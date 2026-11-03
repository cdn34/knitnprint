import { Heart } from 'lucide-react'
import { useFavorites } from '../favorites'
import { useI18n } from '../i18n'
import { CatalogProductGrid } from './catalog-product-grid'

export function AccountFavorites() {
  const { t } = useI18n()
  const { products, status, refresh } = useFavorites()
  return (
    <section className="account-panel account-favorites" id="favorites" aria-labelledby="favorites-title">
      <div className="favorites-heading">
        <div className="account-panel-heading">
          <p className="eyebrow">{t('favorites.eyebrow')}</p>
          <h2 id="favorites-title">
            <Heart size={25} aria-hidden="true" />
            {t('favorites.title')}
            {status === 'ready' && <span className="favorites-count" aria-hidden="true">{products.length}</span>}
          </h2>
        </div>
        <a className="text-link" href="/products">{t('favorites.browse')}</a>
      </div>
      {status === 'loading' ? <p role="status">{t('favorites.loading')}</p>
        : status === 'guest' ? (
          <div className="favorites-empty">
            <p>{t('favorites.sessionExpired')}</p>
            <a className="button button--secondary" href="/account">{t('account.signIn')}</a>
          </div>
        ) : status === 'error' ? (
          <div className="favorites-empty" role="alert">
            <p>{t('favorites.loadError')}</p>
            <button className="button button--secondary" type="button" onClick={() => { void refresh() }}>{t('favorites.retry')}</button>
          </div>
        ) : products.length ? <CatalogProductGrid products={products} /> : (
          <div className="favorites-empty">
            <Heart size={36} aria-hidden="true" />
            <h3>{t('favorites.emptyTitle')}</h3>
            <p>{t('favorites.emptyBody')}</p>
            <a className="button button--secondary" href="/products">{t('favorites.browse')}</a>
          </div>
        )}
    </section>
  )
}
