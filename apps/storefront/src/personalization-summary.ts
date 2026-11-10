import type { useI18n } from './i18n'

export function personalizationSummary(value: unknown, mediaIds: string[], t: ReturnType<typeof useI18n>['t']) {
  if (!value || typeof value !== 'object') return t('personalization.summary')
  const customization = value as { areas?: Array<{ view_id?: unknown; photo?: unknown; text?: { content?: unknown } }>; photo?: unknown; text?: { content?: unknown } }
  if (Array.isArray(customization.areas)) {
    const photoCount = customization.areas.filter((area) => Boolean(area.photo)).length
    const textCount = customization.areas.filter((area) => typeof area.text?.content === 'string').length
    const viewCount = new Set(customization.areas.flatMap((area) => typeof area.view_id === 'string' ? [area.view_id] : [])).size
    const parts = [
      viewCount > 1 ? t('personalization.summarySides', { count: viewCount }) : '',
      t(customization.areas.length === 1 ? 'personalization.summaryArea' : 'personalization.summaryAreas', { count: customization.areas.length }),
    ].filter(Boolean)
    if (photoCount) parts.push(t(photoCount === 1 ? 'personalization.summaryPhoto' : 'personalization.summaryPhotos', { count: photoCount }))
    if (textCount) parts.push(t(textCount === 1 ? 'personalization.summaryText' : 'personalization.summaryTexts', { count: textCount }))
    return `${t('personalization.summary')} · ${parts.join(' · ')}`
  }
  const text = customization.text?.content
  return `${t('personalization.summary')}${mediaIds.length || customization.photo ? t('personalization.summaryWithPhoto') : ''}${typeof text === 'string' ? ` · “${text}”` : ''}`
}
