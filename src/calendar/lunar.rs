use lunar_rust::{
    foto::{FotoRef, FotoRefHelper},
    foto_festival::FotoFestivalRefHelper,
    jie_qi::JieQiRefHelper,
    lunar::{LunarRef, LunarRefHelper},
    nine_star::NineStarRefHelper,
    shu_jiu::ShuJiuRefHelper,
    solar::{self, SolarRef, SolarRefHelper},
    tao::{TaoRef, TaoRefHelper},
    tao_festival::TaoFestivalRefHelper,
};
use time::Date;

use super::{CalendarDayDetail, CalendarDayInfo, CalendarFeature};

pub fn day_info(date: Date, enabled_features: &[CalendarFeature]) -> CalendarDayInfo {
    let solar = solar::from_ymd(
        i64::from(date.year()),
        i64::from(u8::from(date.month())),
        i64::from(date.day()),
    );
    let lunar = solar.get_lunar();
    let foto = lunar.get_foto();
    let tao = lunar.get_tao();
    let mut details = Vec::with_capacity(CalendarFeature::ALL.len());
    let enabled = |feature| enabled_features.contains(&feature);
    let leap_month = lunar.get_month() < 0;

    macro_rules! selected_detail {
        ($details:expr, $feature:expr, $values:expr, $short_value:expr $(,)?) => {{
            let feature = $feature;
            if enabled(feature) && is_supported_for_date(feature, leap_month) {
                push_detail($details, feature, $values, $short_value);
            }
        }};
    }

    let lunar_date = lunar.to_string();
    let lunar_short = if lunar.get_day() == 1 {
        format!("{}月", lunar.get_month_in_chinese())
    } else {
        lunar.get_day_in_chinese()
    };
    selected_detail!(
        &mut details,
        CalendarFeature::LunarDate,
        vec![lunar_date],
        Some(lunar_short),
    );
    selected_detail!(
        &mut details,
        CalendarFeature::Weekday,
        vec![format!("星期{}", lunar.get_week_in_chinese())],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::LeapYear,
        vec![
            if solar.is_leap_year() {
                "闰年"
            } else {
                "平年"
            }
            .into()
        ],
        None,
    );

    let solar_festivals = solar.clone().get_festivals();
    selected_detail!(
        &mut details,
        CalendarFeature::SolarFestival,
        solar_festivals.clone(),
        solar_festivals.first().cloned(),
    );
    selected_detail!(
        &mut details,
        CalendarFeature::SolarOtherFestival,
        solar.get_other_festivals(),
        None,
    );
    if enabled(CalendarFeature::HolidayAdjustment) {
        let adjustment = holiday_adjustment(&solar);
        push_detail(
            &mut details,
            CalendarFeature::HolidayAdjustment,
            adjustment.iter().map(|(value, _)| value.clone()).collect(),
            adjustment.first().map(|(_, short)| short.clone()),
        );
    }
    selected_detail!(
        &mut details,
        CalendarFeature::Constellation,
        vec![format!("{}座", solar.get_xing_zuo())],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::JulianDay,
        vec![format!("{:.5}", solar.get_julian_day())],
        None,
    );

    selected_detail!(
        &mut details,
        CalendarFeature::GanZhiYear,
        vec![lunar.get_year_in_gan_zhi()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::GanZhiMonth,
        vec![lunar.get_month_in_gan_zhi()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::GanZhiDay,
        vec![lunar.get_day_in_gan_zhi()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::ZodiacYear,
        vec![lunar.get_year_sheng_xiao()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::ZodiacMonth,
        vec![lunar.get_month_sheng_xiao()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::ZodiacDay,
        vec![lunar.get_day_sheng_xiao()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::Season,
        vec![lunar_season(lunar.get_month())],
        None,
    );

    let solar_term = lunar.get_jie_qi();
    selected_detail!(
        &mut details,
        CalendarFeature::SolarTerm,
        vec![solar_term.clone()],
        non_empty(solar_term),
    );
    selected_detail!(
        &mut details,
        CalendarFeature::NearSolarTerms,
        near_solar_terms(&lunar),
        None,
    );

    let lunar_festivals = lunar.get_festivals();
    selected_detail!(
        &mut details,
        CalendarFeature::LunarFestival,
        lunar_festivals.clone(),
        lunar_festivals.first().cloned(),
    );
    selected_detail!(
        &mut details,
        CalendarFeature::LunarOtherFestival,
        lunar.get_other_festivals(),
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::MoonPhase,
        vec![lunar.get_yue_xiang()],
        None,
    );

    selected_detail!(
        &mut details,
        CalendarFeature::PengZu,
        vec![lunar.get_peng_zu_gan(), lunar.get_peng_zu_zhi()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::DayYi,
        lunar.get_day_yi(None),
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::DayJi,
        lunar.get_day_ji(None),
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::AuspiciousGods,
        lunar.get_day_ji_shen(),
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::InauspiciousSpirits,
        lunar.get_day_xiong_sha(),
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::JoyPosition,
        vec![position(
            lunar.get_day_position_xi(),
            lunar.get_day_position_xi_desc(),
        )],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::YangNoblePosition,
        vec![position(
            lunar.get_day_position_yang_gui(),
            lunar.get_day_position_yang_gui_desc(),
        )],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::YinNoblePosition,
        vec![position(
            lunar.get_day_position_yin_gui(),
            lunar.get_day_position_yin_gui_desc(),
        )],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::FortunePosition,
        vec![position(
            lunar.get_day_position_fu(None),
            lunar.get_day_position_fu_desc(None),
        )],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::WealthPosition,
        vec![position(
            lunar.get_day_position_cai(),
            lunar.get_day_position_cai_desc(),
        )],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::YearTaiSui,
        vec![position(
            lunar.get_year_position_tai_sui(None),
            lunar.get_year_position_tai_sui_desc(None),
        )],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::MonthTaiSui,
        vec![position(
            lunar.get_month_position_tai_sui(None),
            lunar.get_month_position_tai_sui_desc(None),
        )],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::DayTaiSui,
        vec![position(
            lunar.get_day_position_tai_sui(None),
            lunar.get_day_position_tai_sui_desc(None),
        )],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::DayFetalGod,
        vec![lunar.get_day_position_tai()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::MonthFetalGod,
        vec![lunar.get_month_position_tai()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::Chong,
        vec![lunar.get_day_chong_desc()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::Sha,
        vec![lunar.get_day_sha()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::YearNaYin,
        vec![lunar.get_year_na_yin()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::MonthNaYin,
        vec![lunar.get_month_na_yin()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::DayNaYin,
        vec![lunar.get_day_na_yin()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::Xiu,
        vec![
            format!(
                "{}{} · {}",
                lunar.get_xiu(),
                lunar.get_animal(),
                lunar.get_xiu_luck()
            ),
            lunar.get_xiu_song(),
            lunar.get_zheng(),
            lunar.get_gong(),
            lunar.get_shou(),
        ],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::TwelveOfficer,
        vec![lunar.get_zhi_xing()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::DayGod,
        vec![
            lunar.get_day_tian_shen(),
            lunar.get_day_tian_shen_type(),
            lunar.get_day_tian_shen_luck(),
        ],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::YearNineStar,
        vec![lunar.get_year_nine_star(None).to_full_string()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::MonthNineStar,
        vec![lunar.get_month_nine_star(None).to_full_string()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::DayNineStar,
        vec![lunar.get_day_nine_star().to_full_string()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::Xun,
        vec![
            lunar.get_year_xun(),
            lunar.get_month_xun(),
            lunar.get_day_xun()
        ],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::XunKong,
        vec![
            lunar.get_year_xun_kong(),
            lunar.get_month_xun_kong(),
            lunar.get_day_xun_kong(),
        ],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::ShuJiu,
        lunar
            .get_shu_jiu()
            .map(|value| vec![value.to_full_string()])
            .unwrap_or_default(),
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::SanFu,
        san_fu(&lunar, &solar),
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::LiuYao,
        vec![liu_yao(lunar.get_month(), lunar.get_day())],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::WuHou,
        vec![lunar.get_wu_hou()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::Hou,
        vec![lunar.get_hou()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::DayLu,
        vec![lunar.get_day_lu()],
        None,
    );

    selected_detail!(
        &mut details,
        CalendarFeature::BuddhistCalendar,
        vec![foto.to_string()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::BuddhistFestivals,
        foto.get_festivals()
            .into_iter()
            .map(|festival| festival.to_full_string())
            .chain(foto.get_other_festivals())
            .collect(),
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::BuddhistObservances,
        buddhist_observances(&foto),
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::BuddhistXiu,
        vec![
            format!(
                "{}{} · {}",
                foto.get_xiu(),
                foto.get_animal(),
                foto.get_xiu_luck()
            ),
            foto.get_xiu_song(),
            foto.get_zheng(),
            foto.get_gong(),
            foto.get_shou(),
        ],
        None,
    );

    selected_detail!(
        &mut details,
        CalendarFeature::TaoistCalendar,
        vec![tao.to_string()],
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::TaoistFestivals,
        tao.get_festivals()
            .into_iter()
            .map(|festival| festival.to_full_string())
            .collect(),
        None,
    );
    selected_detail!(
        &mut details,
        CalendarFeature::TaoistObservances,
        taoist_observances(&tao),
        None,
    );

    CalendarDayInfo {
        date: solar.to_ymd(),
        details,
    }
}

fn push_detail(
    details: &mut Vec<CalendarDayDetail>,
    feature: CalendarFeature,
    mut values: Vec<String>,
    short_value: Option<String>,
) {
    values.retain(|value| !value.trim().is_empty());
    values.dedup();
    if values.is_empty() {
        return;
    }
    details.push(CalendarDayDetail {
        feature,
        values,
        short_value: short_value.filter(|value| !value.trim().is_empty()),
    });
}

fn push_flag(values: &mut Vec<String>, enabled: bool, label: &str) {
    if enabled {
        values.push(label.into());
    }
}

fn near_solar_terms(lunar: &LunarRef) -> Vec<String> {
    let mut values = Vec::with_capacity(2);
    if let Some(term) = lunar.get_prev_jie_qi(Some(true)) {
        values.push(format!(
            "{} · {}",
            term.get_name(),
            term.get_solar().to_ymd()
        ));
    }
    if let Some(term) = lunar.get_next_jie_qi(Some(true)) {
        values.push(format!(
            "{} · {}",
            term.get_name(),
            term.get_solar().to_ymd()
        ));
    }
    values
}

fn san_fu(lunar: &LunarRef, current: &SolarRef) -> Vec<String> {
    let terms = lunar.get_jie_qi_table();
    let Some(xia_zhi) = terms.get("夏至") else {
        return Vec::new();
    };
    let Some(li_qiu) = terms.get("立秋") else {
        return Vec::new();
    };
    let mut start = solar::from_ymd(xia_zhi.get_year(), xia_zhi.get_month(), xia_zhi.get_day());
    let mut add = 6 - xia_zhi.get_lunar().get_day_gan_index();
    if add < 0 {
        add += 10;
    }
    start = start.next(add + 20, None);
    if current.is_before(start.clone()) {
        return Vec::new();
    }
    let mut days = current.subtract(start.clone());
    if days < 10 {
        return vec![format!("初伏第{}天", days + 1)];
    }
    start = start.next(10, None);
    days = current.subtract(start.clone());
    if days < 10 {
        return vec![format!("中伏第{}天", days + 1)];
    }
    start = start.next(10, None);
    days = current.subtract(start.clone());
    let li_qiu_solar = solar::from_ymd(li_qiu.get_year(), li_qiu.get_month(), li_qiu.get_day());
    if li_qiu_solar.is_after(start.clone()) {
        if days < 10 {
            return vec![format!("中伏第{}天", days + 11)];
        }
        start = start.next(10, None);
        days = current.subtract(start);
    }
    if days < 10 {
        vec![format!("末伏第{}天", days + 1)]
    } else {
        Vec::new()
    }
}

fn buddhist_observances(foto: &FotoRef) -> Vec<String> {
    let mut values = Vec::new();
    push_flag(&mut values, foto.is_month_zhai(), "六斋月");
    push_flag(&mut values, foto.is_day_yang_gong(), "杨公忌");
    push_flag(&mut values, foto.is_day_zhai_shuo_wang(), "朔望斋");
    push_flag(&mut values, foto.is_day_zhai_six(), "六斋日");
    push_flag(&mut values, foto.is_day_zhai_ten(), "十斋日");
    push_flag(&mut values, foto.is_day_zhai_guan_yin(), "观音斋");
    values
}

fn taoist_observances(tao: &TaoRef) -> Vec<String> {
    let mut values = Vec::new();
    push_flag(&mut values, tao.is_day_san_hui(), "三会日");
    push_flag(&mut values, tao.is_day_san_yuan(), "三元日");
    push_flag(&mut values, tao.is_day_ba_jie(), "八节日");
    push_flag(&mut values, tao.is_day_wu_la(), "五腊日");
    push_flag(&mut values, tao.is_day_ba_hui(), "八会日");
    push_flag(&mut values, tao.is_day_ming_wu(), "明戊日");
    if tao.get_month() > 0 {
        push_flag(&mut values, tao.is_day_an_wu(), "暗戊日");
        push_flag(&mut values, tao.is_day_wu(), "戊日");
    } else {
        push_flag(&mut values, tao.is_day_ming_wu(), "戊日");
    }
    push_flag(&mut values, tao.is_day_tian_she(), "天赦日");
    values
}

fn position(value: String, description: String) -> String {
    if description.is_empty() {
        value
    } else {
        format!("{value}（{description}）")
    }
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn lunar_season(month: i64) -> String {
    const SEASONS: [&str; 13] = [
        "", "孟春", "仲春", "季春", "孟夏", "仲夏", "季夏", "孟秋", "仲秋", "季秋", "孟冬", "仲冬",
        "季冬",
    ];
    SEASONS[month.unsigned_abs() as usize].into()
}

fn liu_yao(month: i64, day: i64) -> String {
    const VALUES: [&str; 6] = ["先胜", "友引", "先负", "佛灭", "大安", "赤口"];
    VALUES[((month.unsigned_abs() + day as u64 - 2) % 6) as usize].into()
}

fn is_supported_for_date(feature: CalendarFeature, leap_month: bool) -> bool {
    !leap_month
        || !matches!(
            feature,
            CalendarFeature::AuspiciousGods
                | CalendarFeature::InauspiciousSpirits
                | CalendarFeature::BuddhistXiu
        )
}

fn holiday_adjustment(solar: &SolarRef) -> Vec<(String, String)> {
    let previous_workday = solar.next(-1, Some(true));
    let is_workday = previous_workday.next(1, Some(true)).to_ymd() == solar.to_ymd();
    let weekend = matches!(solar.get_week(), 0 | 6);
    match (weekend, is_workday) {
        (true, true) => vec![("调休上班".into(), "班".into())],
        (false, false) => vec![("法定休息".into(), "休".into())],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_leap_lunar_date() {
        let info = day_info(
            Date::from_calendar_date(2020, time::Month::May, 24).unwrap(),
            &[CalendarFeature::LunarDate],
        );
        let detail = detail(&info, CalendarFeature::LunarDate);
        assert_eq!(detail.values, ["二〇二〇年闰四月初二"]);
        assert_eq!(detail.short_value.as_deref(), Some("初二"));
    }

    #[test]
    fn renders_solar_term_and_lunar_festival() {
        let winter_solstice = day_info(
            Date::from_calendar_date(2021, time::Month::December, 21).unwrap(),
            &[CalendarFeature::SolarTerm],
        );
        assert_eq!(
            detail(&winter_solstice, CalendarFeature::SolarTerm).values,
            ["冬至"]
        );

        let new_year_eve = day_info(
            Date::from_calendar_date(2022, time::Month::January, 31).unwrap(),
            &[CalendarFeature::LunarFestival],
        );
        assert!(
            detail(&new_year_eve, CalendarFeature::LunarFestival)
                .values
                .iter()
                .any(|value| value == "除夕")
        );
    }

    #[test]
    fn all_features_are_safe_for_regular_and_leap_month_dates() {
        for date in [
            Date::from_calendar_date(2024, time::Month::February, 10).unwrap(),
            Date::from_calendar_date(2020, time::Month::May, 24).unwrap(),
        ] {
            for feature in CalendarFeature::ALL {
                let result = std::panic::catch_unwind(|| day_info(date, &[feature]));
                assert!(result.is_ok(), "{feature:?} panicked for {date}");
            }
        }
    }

    fn detail(info: &CalendarDayInfo, feature: CalendarFeature) -> &CalendarDayDetail {
        info.details
            .iter()
            .find(|detail| detail.feature == feature)
            .expect("calendar feature should be present")
    }
}
