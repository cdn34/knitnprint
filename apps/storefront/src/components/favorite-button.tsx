import { Heart } from 'lucide-react'
import { useState } from 'react'
import { requestFavoriteLogin, useFavorites } from '../favorites'
import { useI18n } from '../i18n'

export function FavoriteButton({ productId, title }: Readonly<{ productId: string; title: string }>) {
  const { t } = useI18n()
  const favorites = useFavorites()
  const [error, setError] = useState(false)
  const saved = favorites.products.some(product => product.id === productId)
  const pending = favorites.pendingIds.has(productId)

  async function toggle() {
    setError(false)
    if (favorites.status === 'guest') {
      requestFavoriteLogin(productId)
      return
    }
    const result = await favorites.setSaved(productId, !saved)
    if (result === 'guest') requestFavoriteLogin(productId)
    else setError(result === 'error')
  }

  return <>
    <button
      className={`heart${saved ? ' heart--saved' : ''}`}
      type="button"
      aria-label={t(saved ? 'favorites.remove' : 'favorites.save', { name: title })}
      aria-pressed={saved}
      aria-busy={pending}
      disabled={favorites.status === 'loading' || pending}
      onClick={() => { void toggle() }}
    >
      <Heart size={20} fill={saved ? 'currentColor' : 'none'} aria-hidden="true" />
    </button>
    {error && <span className="favorite-error" role="alert">{t('favorites.saveError')}</span>}
  </>
}
