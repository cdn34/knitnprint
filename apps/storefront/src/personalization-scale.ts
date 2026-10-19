export type PersonalizationFrame = { x: number; y: number; width: number; height: number }
type PhysicalSize = { physicalWidthCm: number; physicalHeightCm: number }

export function printAreaForVariant(
  area: PersonalizationFrame & PhysicalSize,
  baseArticle: PersonalizationFrame & PhysicalSize,
  selectedArticle: PhysicalSize,
) {
  const availableWidth = selectedArticle.physicalWidthCm * area.width / baseArticle.width
  const availableHeight = selectedArticle.physicalHeightCm * area.height / baseArticle.height
  const fittedWidth = Math.min(area.physicalWidthCm, availableWidth)
  const fittedHeight = Math.min(area.physicalHeightCm, availableHeight)
  const previewWidth = area.width * fittedWidth / availableWidth
  const previewHeight = area.height * fittedHeight / availableHeight

  return {
    physicalWidthCm: Math.round(fittedWidth * 100) / 100,
    physicalHeightCm: Math.round(fittedHeight * 100) / 100,
    previewFrame: {
      x: area.x + (area.width - previewWidth) / 2,
      y: area.y,
      width: previewWidth,
      height: previewHeight,
    },
  }
}
