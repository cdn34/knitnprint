import type { PersonalizationConfig } from '@knitnprint/api-client'
import { printAreaForVariant } from './personalization-scale.ts'

const safeBasisPoints = (value: unknown, fallback: number) => typeof value === 'number' && Number.isFinite(value) ? value : fallback

export type ElementFrame = { x: number; y: number; width: number; height: number }
export type PrintArea = ElementFrame & { id: string; label: string; physicalWidthCm: number; physicalHeightCm: number; previewFrame?: ElementFrame }
type ArticleReference = ElementFrame & { physicalWidthCm: number; physicalHeightCm: number }
export type PrintView = { id: string; label: string; mode: string; mediaId?: string; articleReference?: ArticleReference; printAreas: PrintArea[] }

function configuredPrintAreas(raw: unknown, config: PersonalizationConfig, areaLabel: (number: number) => string): PrintArea[] {
  if (Array.isArray(raw)) {
    const areas = raw.flatMap((item, index) => {
      if (!item || typeof item !== 'object') return []
      const area = item as Record<string, unknown>
      const coordinates = ['x', 'y', 'width', 'height'].map((key) => area[key])
      if (coordinates.some((value) => typeof value !== 'number' || !Number.isFinite(value))) return []
      return [{
        id: typeof area.id === 'string' && area.id ? area.id : `area-${index + 1}`,
        label: typeof area.label === 'string' && area.label ? area.label : areaLabel(index + 1),
        x: Number(area.x) / 100,
        y: Number(area.y) / 100,
        width: Number(area.width) / 100,
        height: Number(area.height) / 100,
        physicalWidthCm: typeof area.physical_width_cm === 'number' && Number.isFinite(area.physical_width_cm) ? area.physical_width_cm : 20,
        physicalHeightCm: typeof area.physical_height_cm === 'number' && Number.isFinite(area.physical_height_cm) ? area.physical_height_cm : 20,
      }]
    })
    return areas
  }
  return [{ id: 'area-1', label: areaLabel(1), x: safeBasisPoints(config.area_x, 2500) / 100, y: safeBasisPoints(config.area_y, 2500) / 100, width: safeBasisPoints(config.area_width, 5000) / 100, height: safeBasisPoints(config.area_height, 5000) / 100, physicalWidthCm: 20, physicalHeightCm: 20 }]
}

export function activePersonalizationViews(config: PersonalizationConfig): Record<string, unknown>[] {
  if (!Array.isArray(config.views)) return []
  return config.views.filter((raw): raw is Record<string, unknown> => {
    if (!raw || typeof raw !== 'object') return false
    const view = raw as Record<string, unknown>
    return (view.mode ?? config.mode) !== 'none'
      && Array.isArray(view.print_areas)
      && view.print_areas.length > 0
  })
}

export function configuredViews(config: PersonalizationConfig, variantId: string, frontLabel: string, viewLabel: (number: number) => string, areaLabel: (number: number) => string, productMedia: ReadonlyArray<{ id: string }> = []): PrintView[] {
  if (Array.isArray(config.views)) {
    const views = config.views.flatMap((item, index) => {
      if (!item || typeof item !== 'object') return []
      const view = item as Record<string, unknown>
      const mode = typeof view.mode === 'string' ? view.mode : config.mode
      if (mode === 'none') return []
      const printAreas = configuredPrintAreas(view.print_areas, config, areaLabel)
      if (!printAreas.length) return []
      const rawReference = view.article_reference && typeof view.article_reference === 'object' ? view.article_reference as Record<string, unknown> : undefined
      const referenceCoordinates = rawReference ? ['x', 'y', 'width', 'height'].map((key) => rawReference[key]) : []
      const articleReference = rawReference?.configured === true
        && referenceCoordinates.every((value) => typeof value === 'number' && Number.isFinite(value))
        && typeof rawReference.physical_width_cm === 'number' && Number.isFinite(rawReference.physical_width_cm)
        && typeof rawReference.physical_height_cm === 'number' && Number.isFinite(rawReference.physical_height_cm)
        ? { x: Number(rawReference.x) / 100, y: Number(rawReference.y) / 100, width: Number(rawReference.width) / 100, height: Number(rawReference.height) / 100, physicalWidthCm: rawReference.physical_width_cm, physicalHeightCm: rawReference.physical_height_cm }
        : undefined
      const variantReferences = Array.isArray(view.variant_references) ? view.variant_references : []
      const variantReference = variantReferences.find((raw): raw is Record<string, unknown> => Boolean(raw && typeof raw === 'object' && (raw as Record<string, unknown>).variant_id === variantId && (raw as Record<string, unknown>).configured === true))
      const effectiveReference = articleReference && variantReference
        && typeof variantReference.physical_width_cm === 'number' && typeof variantReference.physical_height_cm === 'number'
        ? { ...articleReference, physicalWidthCm: variantReference.physical_width_cm, physicalHeightCm: variantReference.physical_height_cm }
        : articleReference
      const effectivePrintAreas = articleReference && effectiveReference && variantReference
        ? printAreas.map((area) => {
          return { ...area, ...printAreaForVariant(area, articleReference, effectiveReference) }
        })
        : printAreas
      return [{
        mode,
        id: typeof view.id === 'string' && view.id ? view.id : `view-${index + 1}`,
        label: typeof view.label === 'string' && view.label ? view.label : index === 0 ? frontLabel : viewLabel(index + 1),
        mediaId: typeof view.media_id === 'string' ? view.media_id : undefined,
        articleReference: effectiveReference,
        printAreas: effectivePrintAreas,
      }]
    })
    const photoPositions = new Map(productMedia.map((media, index) => [media.id, index]))
    // Reorder the views by photograph identity, preserving their areas and permissions.
    return views.sort((left, right) =>
      (photoPositions.get(left.mediaId ?? '') ?? productMedia.length)
      - (photoPositions.get(right.mediaId ?? '') ?? productMedia.length),
    )
  }
  return [{ id: 'view-front', mode: config.mode, label: frontLabel, mediaId: config.preview_media_id ?? undefined, printAreas: configuredPrintAreas(config.print_areas, config, areaLabel) }]
}
