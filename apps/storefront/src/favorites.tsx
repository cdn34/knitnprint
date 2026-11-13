import { ApiError, createApiClient, type Product } from '@knitnprint/api-client'
import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from 'react'

const api = createApiClient()
const pendingFavoriteKey = 'knitnprint-pending-favorite'
type Status = 'loading' | 'ready' | 'guest' | 'error'
type SaveResult = 'saved' | 'guest' | 'error'

export function pendingFavorite(): string | null {
  try { return sessionStorage.getItem(pendingFavoriteKey) } catch { return null }
}

export function requestFavoriteLogin(productId: string) {
  try { sessionStorage.setItem(pendingFavoriteKey, productId) } catch { /* Sign-in remains available without storage. */ }
  window.location.assign('/account#favorites')
}

const FavoritesContext = createContext<{
  products: Product[]
  status: Status
  pendingIds: Set<string>
  refresh: () => Promise<Status | undefined>
  clear: () => void
  setSaved: (id: string, saved: boolean) => Promise<SaveResult>
  completePending: () => Promise<SaveResult | null>
} | null>(null)

export function FavoritesProvider({ children }: Readonly<{ children: ReactNode }>) {
  const [products, setProducts] = useState<Product[]>([])
  const [status, setStatus] = useState<Status>('loading')
  const [pendingIds, setPendingIds] = useState(new Set<string>())
  const busy = useRef(new Set<string>())
  const generation = useRef(0)

  const refresh = useCallback(async () => {
    const request = ++generation.current
    try {
      const saved = await api.customerFavorites()
      if (request !== generation.current) return
      setProducts(saved)
      setStatus('ready')
      return 'ready' as const
    } catch (cause) {
      if (request !== generation.current) return
      const nextStatus = cause instanceof ApiError && cause.status === 401 ? 'guest' : 'error'
      if (nextStatus === 'guest') setProducts([])
      setStatus(nextStatus)
      return nextStatus
    }
  }, [])

  const clear = useCallback(() => {
    generation.current++
    setProducts([])
    setStatus('guest')
    try { sessionStorage.removeItem(pendingFavoriteKey) } catch { /* Storage may be disabled. */ }
  }, [])

  const setSaved = useCallback(async (id: string, saved: boolean): Promise<SaveResult> => {
    if (busy.current.has(id)) return 'error'
    busy.current.add(id)
    setPendingIds(new Set(busy.current))
    try {
      if (saved) await api.saveCustomerFavorite(id)
      else await api.removeCustomerFavorite(id)
      const result = await refresh()
      return result === 'guest' ? 'guest' : result === 'error' ? 'error' : 'saved'
    } catch (cause) {
      if (cause instanceof ApiError && cause.status === 401) {
        clear()
        return 'guest'
      }
      return 'error'
    } finally {
      busy.current.delete(id)
      setPendingIds(new Set(busy.current))
    }
  }, [clear, refresh])

  const completePending = useCallback(async () => {
    const id = pendingFavorite()
    if (!id) return null
    const result = await setSaved(id, true)
    if (result === 'saved') {
      try { sessionStorage.removeItem(pendingFavoriteKey) } catch { /* Storage may be disabled. */ }
    }
    return result
  }, [setSaved])

  useEffect(() => {
    void refresh()
    const onFocus = () => { void refresh() }
    window.addEventListener('focus', onFocus)
    return () => {
      generation.current++
      window.removeEventListener('focus', onFocus)
    }
  }, [refresh])

  return <FavoritesContext.Provider value={{ products, status, pendingIds, refresh, clear, setSaved, completePending }}>{children}</FavoritesContext.Provider>
}

export function useFavorites() {
  const context = useContext(FavoritesContext)
  if (!context) throw new Error('FavoritesProvider is required')
  return context
}
