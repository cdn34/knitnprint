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
