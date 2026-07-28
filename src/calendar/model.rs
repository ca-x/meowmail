use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarAccount {
    pub id: Uuid,
    pub name: String,
    pub base_url: String,
    pub username: String,
    pub enabled: bool,
    pub has_password: bool,
    pub last_synced_at: Option<i64>,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarAccountInput {
    pub name: String,
    pub base_url: String,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: Uuid,
    pub account_id: Uuid,
    pub display_name: String,
    pub color: String,
    pub remote_href: String,
    pub sync_token: Option<String>,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarUpdate {
    pub display_name: String,
    pub color: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub location: String,
    pub starts_at: i64,
    pub ends_at: i64,
    pub all_day: bool,
    pub timezone: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum CalendarFeature {
    LunarDate,
    Weekday,
    LeapYear,
    SolarFestival,
    SolarOtherFestival,
    HolidayAdjustment,
    Constellation,
    JulianDay,
    GanZhiYear,
    GanZhiMonth,
    GanZhiDay,
    ZodiacYear,
    ZodiacMonth,
    ZodiacDay,
    Season,
    SolarTerm,
    NearSolarTerms,
    LunarFestival,
    LunarOtherFestival,
    MoonPhase,
    PengZu,
    DayYi,
    DayJi,
    AuspiciousGods,
    InauspiciousSpirits,
    JoyPosition,
    YangNoblePosition,
    YinNoblePosition,
    FortunePosition,
    WealthPosition,
    YearTaiSui,
    MonthTaiSui,
    DayTaiSui,
    DayFetalGod,
    MonthFetalGod,
    Chong,
    Sha,
    YearNaYin,
    MonthNaYin,
    DayNaYin,
    Xiu,
    TwelveOfficer,
    DayGod,
    YearNineStar,
    MonthNineStar,
    DayNineStar,
    Xun,
    XunKong,
    ShuJiu,
    SanFu,
    LiuYao,
    WuHou,
    Hou,
    DayLu,
    BuddhistCalendar,
    BuddhistFestivals,
    BuddhistObservances,
    BuddhistXiu,
    TaoistCalendar,
    TaoistFestivals,
    TaoistObservances,
}

impl CalendarFeature {
    pub const ALL: [Self; 61] = [
        Self::LunarDate,
        Self::Weekday,
        Self::LeapYear,
        Self::SolarFestival,
        Self::SolarOtherFestival,
        Self::HolidayAdjustment,
        Self::Constellation,
        Self::JulianDay,
        Self::GanZhiYear,
        Self::GanZhiMonth,
        Self::GanZhiDay,
        Self::ZodiacYear,
        Self::ZodiacMonth,
        Self::ZodiacDay,
        Self::Season,
        Self::SolarTerm,
        Self::NearSolarTerms,
        Self::LunarFestival,
        Self::LunarOtherFestival,
        Self::MoonPhase,
        Self::PengZu,
        Self::DayYi,
        Self::DayJi,
        Self::AuspiciousGods,
        Self::InauspiciousSpirits,
        Self::JoyPosition,
        Self::YangNoblePosition,
        Self::YinNoblePosition,
        Self::FortunePosition,
        Self::WealthPosition,
        Self::YearTaiSui,
        Self::MonthTaiSui,
        Self::DayTaiSui,
        Self::DayFetalGod,
        Self::MonthFetalGod,
        Self::Chong,
        Self::Sha,
        Self::YearNaYin,
        Self::MonthNaYin,
        Self::DayNaYin,
        Self::Xiu,
        Self::TwelveOfficer,
        Self::DayGod,
        Self::YearNineStar,
        Self::MonthNineStar,
        Self::DayNineStar,
        Self::Xun,
        Self::XunKong,
        Self::ShuJiu,
        Self::SanFu,
        Self::LiuYao,
        Self::WuHou,
        Self::Hou,
        Self::DayLu,
        Self::BuddhistCalendar,
        Self::BuddhistFestivals,
        Self::BuddhistObservances,
        Self::BuddhistXiu,
        Self::TaoistCalendar,
        Self::TaoistFestivals,
        Self::TaoistObservances,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct CalendarPreferences {
    pub enabled_features: Vec<CalendarFeature>,
}

impl Default for CalendarPreferences {
    fn default() -> Self {
        Self {
            enabled_features: vec![
                CalendarFeature::LunarDate,
                CalendarFeature::SolarFestival,
                CalendarFeature::HolidayAdjustment,
                CalendarFeature::SolarTerm,
                CalendarFeature::LunarFestival,
            ],
        }
    }
}

impl CalendarPreferences {
    pub fn normalize(&mut self) {
        self.enabled_features.sort_unstable();
        self.enabled_features.dedup();
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDayInfo {
    pub date: String,
    pub details: Vec<CalendarDayDetail>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDayDetail {
    pub feature: CalendarFeature,
    pub values: Vec<String>,
    pub short_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedEvent {
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub location: String,
    pub starts_at: i64,
    pub ends_at: i64,
    pub all_day: bool,
    pub timezone: Option<String>,
}

fn enabled_default() -> bool {
    true
}

impl CalendarAccountInput {
    pub fn normalize(&mut self, require_password: bool) -> Result<(), AppError> {
        self.name = clean_required(&self.name, "calendar account name", 120)?;
        self.base_url = clean_url(&self.base_url)?;
        self.username = clean_required(&self.username, "calendar username", 320)?;
        self.password = self
            .password
            .take()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if self.password.as_ref().is_some_and(|value| {
            value.len() > 4096 || value.chars().any(|character| character == '\0')
        }) {
            return Err(AppError::Validation("calendar password is invalid".into()));
        }
        if require_password && self.password.is_none() {
            return Err(AppError::Validation("calendar password is required".into()));
        }
        Ok(())
    }
}

impl CalendarUpdate {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.display_name = clean_required(&self.display_name, "calendar name", 160)?;
        self.color = self.color.trim().to_owned();
        if self.color.is_empty()
            || self.color.len() > 40
            || self.color.chars().any(|character| character.is_control())
        {
            return Err(AppError::Validation("calendar color is invalid".into()));
        }
        Ok(())
    }
}

fn clean_url(raw: &str) -> Result<String, AppError> {
    let value = raw.trim().trim_end_matches('/');
    let url = url::Url::parse(value)
        .map_err(|_| AppError::Validation("calendar URL is invalid".into()))?;
    if !matches!(url.scheme(), "https" | "http") || url.username() != "" || url.password().is_some()
    {
        return Err(AppError::Validation("calendar URL is invalid".into()));
    }
    Ok(value.to_owned())
}

fn clean_required(value: &str, field: &str, max: usize) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(AppError::Validation(format!("{field} is invalid")));
    }
    Ok(value.to_owned())
}
