import type { CalendarFeature } from "../../app/types"
import type { MessageKey } from "../../i18n/messages"

export const defaultCalendarFeatures: CalendarFeature[] = [
  "lunarDate",
  "solarFestival",
  "holidayAdjustment",
  "solarTerm",
  "lunarFestival",
]

export const calendarCellFeaturePriority: CalendarFeature[] = [
  "solarTerm",
  "holidayAdjustment",
  "lunarFestival",
  "solarFestival",
  "lunarOtherFestival",
  "solarOtherFestival",
  "lunarDate",
]

export const calendarFeatureGroups: Array<{
  key: string
  label: MessageKey
  features: Array<{ value: CalendarFeature; label: MessageKey }>
}> = [
  {
    key: "basic",
    label: "calendarOptionBasic",
    features: [
      ["lunarDate", "calendarFeatureLunarDate"],
      ["weekday", "calendarFeatureWeekday"],
      ["leapYear", "calendarFeatureLeapYear"],
      ["constellation", "calendarFeatureConstellation"],
      ["julianDay", "calendarFeatureJulianDay"],
      ["ganZhiYear", "calendarFeatureGanZhiYear"],
      ["ganZhiMonth", "calendarFeatureGanZhiMonth"],
      ["ganZhiDay", "calendarFeatureGanZhiDay"],
      ["zodiacYear", "calendarFeatureZodiacYear"],
      ["zodiacMonth", "calendarFeatureZodiacMonth"],
      ["zodiacDay", "calendarFeatureZodiacDay"],
      ["season", "calendarFeatureSeason"],
      ["moonPhase", "calendarFeatureMoonPhase"],
    ].map(([value, label]) => ({ value: value as CalendarFeature, label: label as MessageKey })),
  },
  {
    key: "festivals",
    label: "calendarOptionFestivals",
    features: [
      ["solarFestival", "calendarFeatureSolarFestival"],
      ["solarOtherFestival", "calendarFeatureSolarOtherFestival"],
      ["holidayAdjustment", "calendarFeatureHolidayAdjustment"],
      ["solarTerm", "calendarFeatureSolarTerm"],
      ["nearSolarTerms", "calendarFeatureNearSolarTerms"],
      ["lunarFestival", "calendarFeatureLunarFestival"],
      ["lunarOtherFestival", "calendarFeatureLunarOtherFestival"],
      ["shuJiu", "calendarFeatureShuJiu"],
      ["sanFu", "calendarFeatureSanFu"],
      ["wuHou", "calendarFeatureWuHou"],
      ["hou", "calendarFeatureHou"],
    ].map(([value, label]) => ({ value: value as CalendarFeature, label: label as MessageKey })),
  },
  {
    key: "almanac",
    label: "calendarOptionAlmanac",
    features: [
      ["pengZu", "calendarFeaturePengZu"],
      ["dayYi", "calendarFeatureDayYi"],
      ["dayJi", "calendarFeatureDayJi"],
      ["auspiciousGods", "calendarFeatureAuspiciousGods"],
      ["inauspiciousSpirits", "calendarFeatureInauspiciousSpirits"],
      ["joyPosition", "calendarFeatureJoyPosition"],
      ["yangNoblePosition", "calendarFeatureYangNoblePosition"],
      ["yinNoblePosition", "calendarFeatureYinNoblePosition"],
      ["fortunePosition", "calendarFeatureFortunePosition"],
      ["wealthPosition", "calendarFeatureWealthPosition"],
      ["yearTaiSui", "calendarFeatureYearTaiSui"],
      ["monthTaiSui", "calendarFeatureMonthTaiSui"],
      ["dayTaiSui", "calendarFeatureDayTaiSui"],
      ["dayFetalGod", "calendarFeatureDayFetalGod"],
      ["monthFetalGod", "calendarFeatureMonthFetalGod"],
      ["chong", "calendarFeatureChong"],
      ["sha", "calendarFeatureSha"],
      ["yearNaYin", "calendarFeatureYearNaYin"],
      ["monthNaYin", "calendarFeatureMonthNaYin"],
      ["dayNaYin", "calendarFeatureDayNaYin"],
      ["xiu", "calendarFeatureXiu"],
      ["twelveOfficer", "calendarFeatureTwelveOfficer"],
      ["dayGod", "calendarFeatureDayGod"],
      ["yearNineStar", "calendarFeatureYearNineStar"],
      ["monthNineStar", "calendarFeatureMonthNineStar"],
      ["dayNineStar", "calendarFeatureDayNineStar"],
      ["xun", "calendarFeatureXun"],
      ["xunKong", "calendarFeatureXunKong"],
      ["liuYao", "calendarFeatureLiuYao"],
      ["dayLu", "calendarFeatureDayLu"],
    ].map(([value, label]) => ({ value: value as CalendarFeature, label: label as MessageKey })),
  },
  {
    key: "buddhist",
    label: "calendarOptionBuddhist",
    features: [
      ["buddhistCalendar", "calendarFeatureBuddhistCalendar"],
      ["buddhistFestivals", "calendarFeatureBuddhistFestivals"],
      ["buddhistObservances", "calendarFeatureBuddhistObservances"],
      ["buddhistXiu", "calendarFeatureBuddhistXiu"],
    ].map(([value, label]) => ({ value: value as CalendarFeature, label: label as MessageKey })),
  },
  {
    key: "taoist",
    label: "calendarOptionTaoist",
    features: [
      ["taoistCalendar", "calendarFeatureTaoistCalendar"],
      ["taoistFestivals", "calendarFeatureTaoistFestivals"],
      ["taoistObservances", "calendarFeatureTaoistObservances"],
    ].map(([value, label]) => ({ value: value as CalendarFeature, label: label as MessageKey })),
  },
]

export const allCalendarFeatures = calendarFeatureGroups.flatMap((group) => group.features.map((feature) => feature.value))

export const calendarFeatureLabels = new Map(
  calendarFeatureGroups.flatMap((group) => group.features.map((feature) => [feature.value, feature.label] as const)),
)
