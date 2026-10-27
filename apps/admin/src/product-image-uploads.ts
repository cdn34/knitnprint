/** Keep completed uploads across retries, including a failed final product save. */
export async function uploadImagesWithProgress<T extends { id?: string }>(
  images: T[],
  completed: Map<string, string>,
  upload: (image: T) => Promise<string>,
): Promise<boolean> {
  for (const image of images) {
    if (image.id && completed.has(image.id)) continue
    try {
      const mediaId = await upload(image)
      if (image.id) completed.set(image.id, mediaId)
    } catch {
      return false
    }
  }
  return true
}

/** Preserve each photo's identity while allowing new and saved photos to share one order. */
export function productPhotosInOrder<T extends { id: string }>(photos: T[], order: string[]): T[] {
  const available = new Map(photos.map((photo) => [photo.id, photo]))
  const result: T[] = []
  for (const id of [...order, ...photos.map((photo) => photo.id)]) {
    const photo = available.get(id)
    if (!photo) continue
    result.push(photo)
    available.delete(id)
  }
  return result
}

export function savedProductPhotoOrder(order: string[], uploaded: Map<string, string>): string[] {
  return order.map((id) => uploaded.get(id) ?? id)
}
