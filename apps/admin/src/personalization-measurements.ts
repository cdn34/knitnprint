type Frame = { x: number; y: number; width: number; height: number }
type PhysicalSize = { physicalWidthCm: number; physicalHeightCm: number }

export function printAreaOffsets(reference: Frame & PhysicalSize, area: Frame, size: PhysicalSize) {
  const round = (value: number) => Math.round(value * 10) / 10
  const availableWidth = size.physicalWidthCm * area.width / reference.width
  const availableHeight = size.physicalHeightCm * area.height / reference.height
  const maximumWidth = reference.physicalWidthCm * area.width / reference.width
  const maximumHeight = reference.physicalHeightCm * area.height / reference.height
  const printWidth = Math.min(maximumWidth, availableWidth)
  const printHeight = Math.min(maximumHeight, availableHeight)
  const left = size.physicalWidthCm * (area.x - reference.x) / reference.width + Math.max(0, availableWidth - printWidth) / 2
  const top = size.physicalHeightCm * (area.y - reference.y) / reference.height

  return {
    width: round(printWidth),
    height: round(printHeight),
    top: round(top),
    left: round(left),
    right: round(size.physicalWidthCm - left - printWidth),
    bottom: round(size.physicalHeightCm - top - printHeight),
  }
}
