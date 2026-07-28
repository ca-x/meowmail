use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};

use claude_sdk::{ClaudeClient, ContentBlock, Message as ClaudeMessage, MessagesRequest};
use gemini_rust::{GeminiBuilder, Model};
use openai_rust_sdk::{ChatBuilder, OpenAIClient};
use reqwest::{Client, Proxy, redirect::Policy};
use reqwest12::{ClientBuilder as GeminiClientBuilder, Proxy as GeminiProxy};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    accounts::{ProxyConfig, ProxyKind},
    db::entities::ai_provider,
    error::AppError,
    messages::MessageDetail,
};

use super::{
    model::{
        AiApiType, AiProviderKind, AutoLabelRuleFeed, AutoLabelSubscriptionSyncResult, Label,
        validate_ai_text,
    },
    repository::{AiProviderSecrets, AiRepository},
};

const AI_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_AI_OUTPUT: usize = 120 * 1024;
const SUBSCRIPTION_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const SUBSCRIPTION_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_SUBSCRIPTION_SIZE: usize = 1024 * 1024;

#[derive(Clone)]
pub struct AiService {
    repository: AiRepository,
}

impl AiService {
    pub fn new(repository: AiRepository) -> Self {
        Self { repository }
    }

    pub async fn test_provider(
        &self,
        user_id: Uuid,
        provider_id: Uuid,
    ) -> Result<String, AppError> {
        let (provider, secrets, proxy) = self
            .repository
            .get_provider_with_secrets(user_id, provider_id)
            .await?;
        call_ai(
            &provider,
            &secrets,
            &proxy,
            "Reply with exactly: ok",
            "Connectivity test. Return only ok.",
            128,
            0.0,
        )
        .await
    }

    pub async fn translate(
        &self,
        user_id: Uuid,
        provider_id: Option<Uuid>,
        text: &str,
        target_language: Option<&str>,
    ) -> Result<String, AppError> {
        let text = validate_ai_text(text)?;
        let target = super::model::clean_optional_phrase(target_language, "target language", 80)?
            .unwrap_or_else(|| "Chinese".into());
        let (provider, secrets, proxy) = self.resolve_provider(user_id, provider_id).await?;
        let system = format!(
            "Translate the email content to {target}. Preserve names, addresses, links, dates, and formatting. Return translated text only."
        );
        call_ai(&provider, &secrets, &proxy, &system, &text, 4096, 0.2).await
    }

    pub async fn polish(
        &self,
        user_id: Uuid,
        provider_id: Option<Uuid>,
        text: &str,
        tone: Option<&str>,
    ) -> Result<String, AppError> {
        let text = validate_ai_text(text)?;
        let tone = super::model::clean_optional_phrase(tone, "tone", 80)?
            .unwrap_or_else(|| "clear and professional".into());
        let (provider, secrets, proxy) = self.resolve_provider(user_id, provider_id).await?;
        let system = format!(
            "Polish this email in a {tone} tone. Preserve factual meaning, recipients' names, links, and signature. Return the improved email body only."
        );
        call_ai(&provider, &secrets, &proxy, &system, &text, 4096, 0.45).await
    }

    pub async fn classify_message(
        &self,
        user_id: Uuid,
        provider_id: Option<Uuid>,
        message: &MessageDetail,
        labels: &[Label],
        instructions: &str,
    ) -> Result<Vec<Uuid>, AppError> {
        if labels.is_empty() {
            return Ok(Vec::new());
        }
        let (provider, secrets, proxy) = self.resolve_provider(user_id, provider_id).await?;
        let label_list = labels
            .iter()
            .map(|label| format!("- {}: {}", label.id, label.name))
            .collect::<Vec<_>>()
            .join("\n");
        let system = format!(
            "Classify the email into zero or more allowed labels. Return only a JSON array of label id strings. Allowed labels:\n{label_list}\nAdditional user rule: {instructions}"
        );
        let input = format!(
            "From: {}\nSubject: {}\nPreview: {}\n\n{}",
            message.summary.sender_email,
            message.summary.subject,
            message.summary.preview,
            message.body_text
        );
        let raw = call_ai(&provider, &secrets, &proxy, &system, &input, 1024, 0.0).await?;
        let allowed = labels
            .iter()
            .map(|label| label.id)
            .collect::<std::collections::HashSet<_>>();
        let parsed = parse_label_ids(&raw)
            .into_iter()
            .filter(|id| allowed.contains(id))
            .collect::<Vec<_>>();
        Ok(parsed)
    }

    async fn resolve_provider(
        &self,
        user_id: Uuid,
        provider_id: Option<Uuid>,
    ) -> Result<(ai_provider::Model, AiProviderSecrets, ProxyConfig), AppError> {
        match provider_id {
            Some(id) => self.repository.get_provider_with_secrets(user_id, id).await,
            None => self.repository.default_provider(user_id).await,
        }
    }
}

#[derive(Clone)]
pub struct AutoLabelSubscriptionService {
    repository: AiRepository,
}

impl AutoLabelSubscriptionService {
    pub fn new(repository: AiRepository) -> Self {
        Self { repository }
    }

    pub async fn sync(
        &self,
        user_id: Uuid,
        subscription_id: Uuid,
    ) -> Result<AutoLabelSubscriptionSyncResult, AppError> {
        let subscription = self
            .repository
            .get_auto_label_subscription(user_id, subscription_id)
            .await?;
        if !subscription.enabled {
            return Err(AppError::Validation(
                "auto-label subscription is disabled".into(),
            ));
        }
        let result = self.fetch_and_apply(user_id, &subscription).await;
        match result {
            Ok((labels_imported, rules_imported, rules_skipped)) => {
                let subscription = self
                    .repository
                    .record_subscription_sync(user_id, subscription_id, None)
                    .await?;
                Ok(AutoLabelSubscriptionSyncResult {
                    subscription,
                    labels_imported,
                    rules_imported,
                    rules_skipped,
                })
            }
            Err(error) => {
                let _ = self
                    .repository
                    .record_subscription_sync(
                        user_id,
                        subscription_id,
                        Some(subscription_error_message(&error)),
                    )
                    .await;
                Err(error)
            }
        }
    }

    async fn fetch_and_apply(
        &self,
        user_id: Uuid,
        subscription: &super::model::AutoLabelSubscription,
    ) -> Result<(u32, u32, u32), AppError> {
        let url = url::Url::parse(&subscription.url)
            .map_err(|_| AppError::Validation("subscription URL is invalid".into()))?;
        let host = url
            .host_str()
            .ok_or_else(|| AppError::Validation("subscription URL is invalid".into()))?
            .to_owned();
        let port = url
            .port_or_known_default()
            .ok_or_else(|| AppError::Validation("subscription URL port is invalid".into()))?;
        let addresses = resolve_public_addresses(&host, port).await?;
        let client = Client::builder()
            .connect_timeout(SUBSCRIPTION_CONNECT_TIMEOUT)
            .timeout(SUBSCRIPTION_TIMEOUT)
            .redirect(Policy::none())
            .user_agent(concat!("meowmail/", env!("CARGO_PKG_VERSION")))
            .resolve_to_addrs(&host, &addresses)
            .build()
            .map_err(subscription_transport_error)?;
        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(subscription_transport_error)?;
        if response.status().is_redirection() {
            return Err(AppError::Validation(
                "auto-label subscriptions cannot redirect".into(),
            ));
        }
        if !response.status().is_success() {
            return Err(AppError::Ai(format!(
                "auto-label subscription returned HTTP {}",
                response.status()
            )));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_SUBSCRIPTION_SIZE as u64)
        {
            return Err(AppError::Validation(
                "auto-label subscription is too large".into(),
            ));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(subscription_transport_error)?
        {
            if body.len().saturating_add(chunk.len()) > MAX_SUBSCRIPTION_SIZE {
                return Err(AppError::Validation(
                    "auto-label subscription is too large".into(),
                ));
            }
            body.extend_from_slice(&chunk);
        }
        let mut feed: AutoLabelRuleFeed = serde_json::from_slice(&body)
            .map_err(|_| AppError::Validation("auto-label subscription JSON is invalid".into()))?;
        feed.normalize(Some(&subscription.name))?;
        self.repository
            .replace_subscription_rules(user_id, subscription, feed)
            .await
    }
}

async fn resolve_public_addresses(host: &str, port: u16) -> Result<Vec<SocketAddr>, AppError> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !is_public_ip(ip) {
            return Err(AppError::Validation(
                "subscription URL must resolve to a public address".into(),
            ));
        }
        return Ok(vec![SocketAddr::new(ip, port)]);
    }
    let resolved = tokio::time::timeout(
        SUBSCRIPTION_CONNECT_TIMEOUT,
        tokio::net::lookup_host((host, port)),
    )
    .await
    .map_err(|_| AppError::Ai("auto-label subscription DNS lookup timed out".into()))?
    .map_err(subscription_transport_error)?;
    let mut addresses = resolved.collect::<Vec<_>>();
    addresses.sort_unstable();
    addresses.dedup();
    if addresses.is_empty() {
        return Err(AppError::Ai(
            "auto-label subscription host could not be resolved".into(),
        ));
    }
    if addresses.iter().any(|address| !is_public_ip(address.ip())) {
        return Err(AppError::Validation(
            "subscription URL must resolve only to public addresses".into(),
        ));
    }
    Ok(addresses)
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0
        || a == 10
        || a == 127
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 192 && b == 88 && c == 99)
        || (a == 192 && b == 168)
        || (a == 198 && (b == 18 || b == 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(ipv4) = ip.to_ipv4_mapped() {
        return is_public_ipv4(ipv4);
    }
    let segments = ip.segments();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] & 0xffc0) == 0xfec0
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || (segments[0] == 0x2001 && segments[1] == 0x0002)
        || (segments[0] == 0x2001 && (segments[1] & 0xfff0) == 0x0010))
}

fn subscription_transport_error(error: impl std::fmt::Display) -> AppError {
    tracing::warn!(error = %error, "auto-label subscription request failed");
    AppError::Ai("auto-label subscription request failed".into())
}

fn subscription_error_message(error: &AppError) -> String {
    match error {
        AppError::Validation(message) | AppError::Ai(message) => message.clone(),
        _ => "auto-label subscription could not be synchronized".into(),
    }
}

async fn call_ai(
    provider: &ai_provider::Model,
    secrets: &AiProviderSecrets,
    proxy: &ProxyConfig,
    system: &str,
    input: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<String, AppError> {
    let api_key = secrets
        .api_key
        .as_ref()
        .ok_or_else(|| AppError::Validation("AI API key is not configured".into()))?;
    let kind = AiProviderKind::parse(&provider.provider_kind)?;
    let api_type = AiApiType::parse(&provider.api_type)?;
    let output = match (kind, api_type, provider.base_url.as_deref(), proxy.kind) {
        (AiProviderKind::OpenAi, AiApiType::Responses, None, ProxyKind::Direct) => {
            let client = OpenAIClient::new(api_key.expose_secret()).map_err(to_ai_error)?;
            client
                .generate_with_instructions(&provider.model, input, system)
                .await
                .map_err(to_ai_error)?
        }
        (AiProviderKind::OpenAi, AiApiType::Chat, None, ProxyKind::Direct) => {
            let client = OpenAIClient::new(api_key.expose_secret()).map_err(to_ai_error)?;
            let chat = ChatBuilder::new().developer(system).user(input);
            client
                .chat(&provider.model, chat)
                .await
                .map_err(to_ai_error)?
        }
        (AiProviderKind::OpenAi, AiApiType::Responses, Some(base_url), ProxyKind::Direct) => {
            let client = OpenAIClient::with_base_url(api_key.expose_secret(), base_url)
                .map_err(to_ai_error)?;
            client
                .generate_with_instructions(&provider.model, input, system)
                .await
                .map_err(to_ai_error)?
        }
        (AiProviderKind::OpenAi, AiApiType::Chat, Some(base_url), ProxyKind::Direct) => {
            let client = OpenAIClient::with_base_url(api_key.expose_secret(), base_url)
                .map_err(to_ai_error)?;
            let chat = ChatBuilder::new().developer(system).user(input);
            client
                .chat(&provider.model, chat)
                .await
                .map_err(to_ai_error)?
        }
        (AiProviderKind::OpenAi, AiApiType::Responses, _, _) => {
            call_openai_responses_compat(
                provider,
                api_key.expose_secret(),
                proxy,
                system,
                input,
                max_tokens,
            )
            .await?
        }
        (AiProviderKind::OpenAi, AiApiType::Chat, _, _) => {
            call_openai_chat_compat(
                provider,
                api_key.expose_secret(),
                proxy,
                system,
                input,
                max_tokens,
                temperature,
            )
            .await?
        }
        (AiProviderKind::Claude, AiApiType::Messages, None, ProxyKind::Direct) => {
            let client = ClaudeClient::anthropic(api_key.expose_secret());
            let request = MessagesRequest::new(
                provider.model.clone(),
                max_tokens,
                vec![ClaudeMessage::user(input)],
            )
            .with_system(system)
            .with_temperature(temperature);
            let response = client.send_message(request).await.map_err(to_ai_error)?;
            response
                .content
                .into_iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text, .. } => Some(text),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        (AiProviderKind::Claude, AiApiType::Messages, _, _) => {
            call_claude_compat(
                provider,
                api_key.expose_secret(),
                proxy,
                system,
                input,
                max_tokens,
                temperature,
            )
            .await?
        }
        (AiProviderKind::Gemini, AiApiType::GenerateContent, None, ProxyKind::Direct) => {
            let mut builder = GeminiBuilder::new(api_key.expose_secret())
                .with_model(Model::Custom(provider.model.clone()));
            builder = builder.with_http_client(gemini_reqwest_builder_for_proxy(proxy)?);
            let client = builder.build().map_err(to_ai_error)?;
            #[allow(deprecated)]
            client
                .generate_content()
                .with_system_prompt(system)
                .with_user_message(input)
                .with_temperature(temperature)
                .with_max_output_tokens(i32::try_from(max_tokens).unwrap_or(4096))
                .execute()
                .await
                .map_err(to_ai_error)?
                .text()
        }
        (AiProviderKind::Gemini, AiApiType::GenerateContent, _, _) => {
            call_gemini_generate_compat(
                provider,
                api_key.expose_secret(),
                proxy,
                system,
                input,
                max_tokens,
                temperature,
            )
            .await?
        }
        _ => {
            return Err(AppError::Validation(
                "AI provider API type is invalid".into(),
            ));
        }
    };
    normalize_output(output)
}

fn reqwest_builder_for_proxy(proxy: &ProxyConfig) -> Result<reqwest::ClientBuilder, AppError> {
    let mut builder = reqwest::Client::builder()
        .timeout(AI_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(5));
    if proxy.kind != ProxyKind::Direct {
        builder = builder.proxy(proxy_from_config(proxy)?);
    }
    Ok(builder)
}

fn proxy_from_config(proxy: &ProxyConfig) -> Result<Proxy, AppError> {
    let host = proxy
        .host
        .as_deref()
        .ok_or_else(|| AppError::Validation("proxy host is required".into()))?;
    let port = proxy
        .port
        .ok_or_else(|| AppError::Validation("proxy port is required".into()))?;
    let scheme = match proxy.kind {
        ProxyKind::Http => "http",
        ProxyKind::Socks5 => "socks5h",
        ProxyKind::Direct => return Err(AppError::Validation("proxy kind is invalid".into())),
    };
    let url = format!("{scheme}://{host}:{port}");
    let mut reqwest_proxy = Proxy::all(&url).map_err(to_ai_error)?;
    if let Some(username) = proxy.username.as_deref() {
        let password = proxy
            .password
            .as_ref()
            .map_or("", |value| value.expose_secret());
        reqwest_proxy = reqwest_proxy.basic_auth(username, password);
    }
    Ok(reqwest_proxy)
}

fn gemini_reqwest_builder_for_proxy(proxy: &ProxyConfig) -> Result<GeminiClientBuilder, AppError> {
    let mut builder = GeminiClientBuilder::new()
        .timeout(AI_TIMEOUT)
        .redirect(reqwest12::redirect::Policy::limited(5));
    if proxy.kind != ProxyKind::Direct {
        builder = builder.proxy(gemini_proxy_from_config(proxy)?);
    }
    Ok(builder)
}

fn gemini_proxy_from_config(proxy: &ProxyConfig) -> Result<GeminiProxy, AppError> {
    let host = proxy
        .host
        .as_deref()
        .ok_or_else(|| AppError::Validation("proxy host is required".into()))?;
    let port = proxy
        .port
        .ok_or_else(|| AppError::Validation("proxy port is required".into()))?;
    let scheme = match proxy.kind {
        ProxyKind::Http => "http",
        ProxyKind::Socks5 => "socks5h",
        ProxyKind::Direct => return Err(AppError::Validation("proxy kind is invalid".into())),
    };
    let url = format!("{scheme}://{host}:{port}");
    let mut reqwest_proxy = GeminiProxy::all(&url).map_err(to_ai_error)?;
    if let Some(username) = proxy.username.as_deref() {
        let password = proxy
            .password
            .as_ref()
            .map_or("", |value| value.expose_secret());
        reqwest_proxy = reqwest_proxy.basic_auth(username, password);
    }
    Ok(reqwest_proxy)
}

fn http_client(proxy: &ProxyConfig) -> Result<Client, AppError> {
    reqwest_builder_for_proxy(proxy)?
        .build()
        .map_err(to_ai_error)
}

fn endpoint(provider: &ai_provider::Model, default: &str, suffix: &str) -> String {
    let base = provider
        .base_url
        .as_deref()
        .unwrap_or(default)
        .trim_end_matches('/');
    format!("{base}/{suffix}")
}

async fn call_openai_chat_compat(
    provider: &ai_provider::Model,
    api_key: &str,
    proxy: &ProxyConfig,
    system: &str,
    input: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<String, AppError> {
    #[derive(Serialize)]
    struct Request<'a> {
        model: &'a str,
        messages: Vec<ChatMessage<'a>>,
        max_tokens: u32,
        temperature: f32,
    }
    #[derive(Serialize)]
    struct ChatMessage<'a> {
        role: &'a str,
        content: &'a str,
    }
    #[derive(Deserialize)]
    struct Response {
        choices: Vec<Choice>,
    }
    #[derive(Deserialize)]
    struct Choice {
        message: ChoiceMessage,
    }
    #[derive(Deserialize)]
    struct ChoiceMessage {
        content: String,
    }
    let response: Response = http_client(proxy)?
        .post(endpoint(
            provider,
            "https://api.openai.com/v1",
            "chat/completions",
        ))
        .bearer_auth(api_key)
        .json(&Request {
            model: &provider.model,
            messages: vec![
                ChatMessage {
                    role: "developer",
                    content: system,
                },
                ChatMessage {
                    role: "user",
                    content: input,
                },
            ],
            max_tokens,
            temperature,
        })
        .send()
        .await
        .map_err(to_ai_error)?
        .error_for_status()
        .map_err(to_ai_error)?
        .json()
        .await
        .map_err(to_ai_error)?;
    Ok(response
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .unwrap_or_default())
}

async fn call_openai_responses_compat(
    provider: &ai_provider::Model,
    api_key: &str,
    proxy: &ProxyConfig,
    system: &str,
    input: &str,
    max_output_tokens: u32,
) -> Result<String, AppError> {
    #[derive(Serialize)]
    struct Request<'a> {
        model: &'a str,
        instructions: &'a str,
        input: &'a str,
        max_output_tokens: u32,
    }
    let value: serde_json::Value = http_client(proxy)?
        .post(endpoint(provider, "https://api.openai.com/v1", "responses"))
        .bearer_auth(api_key)
        .json(&Request {
            model: &provider.model,
            instructions: system,
            input,
            max_output_tokens,
        })
        .send()
        .await
        .map_err(to_ai_error)?
        .error_for_status()
        .map_err(to_ai_error)?
        .json()
        .await
        .map_err(to_ai_error)?;
    Ok(value
        .get("output_text")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| collect_text_fields(&value)))
}

async fn call_claude_compat(
    provider: &ai_provider::Model,
    api_key: &str,
    proxy: &ProxyConfig,
    system: &str,
    input: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<String, AppError> {
    #[derive(Serialize)]
    struct Request<'a> {
        model: &'a str,
        max_tokens: u32,
        system: &'a str,
        temperature: f32,
        messages: Vec<ClaudeCompatMessage<'a>>,
    }
    #[derive(Serialize)]
    struct ClaudeCompatMessage<'a> {
        role: &'a str,
        content: &'a str,
    }
    #[derive(Deserialize)]
    struct Response {
        content: Vec<ClaudeContent>,
    }
    #[derive(Deserialize)]
    struct ClaudeContent {
        #[serde(rename = "type")]
        kind: String,
        text: Option<String>,
    }
    let response: Response = http_client(proxy)?
        .post(endpoint(
            provider,
            "https://api.anthropic.com/v1",
            "messages",
        ))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&Request {
            model: &provider.model,
            max_tokens,
            system,
            temperature,
            messages: vec![ClaudeCompatMessage {
                role: "user",
                content: input,
            }],
        })
        .send()
        .await
        .map_err(to_ai_error)?
        .error_for_status()
        .map_err(to_ai_error)?
        .json()
        .await
        .map_err(to_ai_error)?;
    Ok(response
        .content
        .into_iter()
        .filter(|item| item.kind == "text")
        .filter_map(|item| item.text)
        .collect::<Vec<_>>()
        .join("\n"))
}

async fn call_gemini_generate_compat(
    provider: &ai_provider::Model,
    api_key: &str,
    proxy: &ProxyConfig,
    system: &str,
    input: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<String, AppError> {
    #[derive(Serialize)]
    struct Request<'a> {
        contents: Vec<GeminiContent<'a>>,
        #[serde(rename = "systemInstruction")]
        system_instruction: GeminiContent<'a>,
        #[serde(rename = "generationConfig")]
        generation_config: GeminiGenerationConfig,
    }
    #[derive(Serialize)]
    struct GeminiContent<'a> {
        role: Option<&'a str>,
        parts: Vec<GeminiPart<'a>>,
    }
    #[derive(Serialize)]
    struct GeminiPart<'a> {
        text: &'a str,
    }
    #[derive(Serialize)]
    struct GeminiGenerationConfig {
        temperature: f32,
        #[serde(rename = "maxOutputTokens")]
        max_output_tokens: u32,
    }
    let value: serde_json::Value = http_client(proxy)?
        .post(gemini_endpoint(provider, api_key)?)
        .json(&Request {
            contents: vec![GeminiContent {
                role: Some("user"),
                parts: vec![GeminiPart { text: input }],
            }],
            system_instruction: GeminiContent {
                role: None,
                parts: vec![GeminiPart { text: system }],
            },
            generation_config: GeminiGenerationConfig {
                temperature,
                max_output_tokens: max_tokens,
            },
        })
        .send()
        .await
        .map_err(to_ai_error)?
        .error_for_status()
        .map_err(to_ai_error)?
        .json()
        .await
        .map_err(to_ai_error)?;
    Ok(gemini_text(&value))
}

fn gemini_endpoint(provider: &ai_provider::Model, api_key: &str) -> Result<String, AppError> {
    let model = provider
        .model
        .strip_prefix("models/")
        .unwrap_or(&provider.model);
    let base = provider
        .base_url
        .as_deref()
        .unwrap_or("https://generativelanguage.googleapis.com/v1beta")
        .trim_end_matches('/');
    Ok(format!(
        "{base}/models/{model}:generateContent?key={}",
        url::form_urlencoded::byte_serialize(api_key.as_bytes()).collect::<String>()
    ))
}

fn gemini_text(value: &serde_json::Value) -> String {
    value
        .get("candidates")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|candidate| {
            candidate
                .get("content")
                .and_then(|content| content.get("parts"))
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|part| part.get("text").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_output(output: String) -> Result<String, AppError> {
    let value = output.trim().to_owned();
    if value.len() > MAX_AI_OUTPUT || value.chars().any(|character| character == '\0') {
        return Err(AppError::Ai("AI output is invalid".into()));
    }
    Ok(value)
}

fn to_ai_error(error: impl std::fmt::Display) -> AppError {
    AppError::Ai(error.to_string())
}

fn collect_text_fields(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => map
            .iter()
            .filter_map(|(key, value)| {
                if key == "text" {
                    value.as_str().map(str::to_owned)
                } else {
                    let nested = collect_text_fields(value);
                    (!nested.is_empty()).then_some(nested)
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        serde_json::Value::Array(items) => items
            .iter()
            .map(collect_text_fields)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn parse_label_ids(raw: &str) -> Vec<Uuid> {
    if let Ok(values) = serde_json::from_str::<Vec<Uuid>>(raw.trim()) {
        return values;
    }
    raw.split(|character: char| {
        character.is_whitespace() || matches!(character, '"' | '\'' | ',' | '[' | ']')
    })
    .filter_map(|part| Uuid::parse_str(part).ok())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::is_public_ip;
    use std::net::IpAddr;

    #[test]
    fn subscription_targets_reject_non_public_addresses() {
        for value in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "172.16.0.1",
            "192.168.0.1",
            "198.18.0.1",
            "224.0.0.1",
            "::",
            "::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(
                !is_public_ip(value.parse::<IpAddr>().expect("valid IP")),
                "{value}"
            );
        }
    }

    #[test]
    fn subscription_targets_allow_public_addresses() {
        for value in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(
                is_public_ip(value.parse::<IpAddr>().expect("valid IP")),
                "{value}"
            );
        }
    }
}
