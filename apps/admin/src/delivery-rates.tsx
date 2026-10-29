import { useEffect, useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { MapPin, Plus, Save, Trash2, Truck } from 'lucide-react'
import { ApiError, createApiClient, type DeliveryQuote, type DeliveryRatesSettings } from '@knitnprint/api-client'

const api = createApiClient()
const key = ['delivery-rates'] as const
type DraftRate = { id: string; country_code: string; region_name: string; postal_prefixes: string; amount: string; active: boolean }
type Draft = { enabled: boolean; revision: number; rates: DraftRate[] }
const draftFrom = (settings: DeliveryRatesSettings): Draft => ({ enabled: settings.enabled, revision: settings.revision, rates: settings.rates.map(rate => ({ id: rate.id, country_code: rate.country_code, region_name: rate.region_name, postal_prefixes: rate.postal_prefixes.join(', '), amount: rate.amount_minor == null ? '' : (rate.amount_minor / 100).toFixed(2), active: rate.active })) })
const countries = new Intl.DisplayNames(['en'], { type: 'region' })
const countryName = (code: string) => countries.of(code) ?? code
const messageFor = (error: unknown) => error instanceof ApiError ? error.body.error.message : 'The request could not be completed. Please try again.'

function minorAmount(value: string): number | null {
  const input = value.trim()
  if (!input) return null
  const match = /^(\d{1,6})(?:[.,](\d{1,2}))?$/.exec(input)
  if (!match) throw new Error('Use a positive price with up to two decimal places, for example 4.50. Leave the price empty if it is not configured.')
  const amount = Number(match[1]) * 100 + Number((match[2] ?? '').padEnd(2, '0'))
  if (amount > 10_000_000) throw new Error('The maximum shipping price is 100,000.00.')
  return amount
}

export function DeliveryRatesManagement() {
  const client = useQueryClient()
  const settings = useQuery({ queryKey: key, queryFn: api.deliveryRates, refetchOnWindowFocus: false })
  const [draft, setDraft] = useState<Draft | null>(null)
  const [reason, setReason] = useState('Update destination delivery prices')
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')
  const [testCountry, setTestCountry] = useState('PT')
  const [testPostal, setTestPostal] = useState('')
  const [quote, setQuote] = useState<DeliveryQuote | null>(null)
  useEffect(() => { if (settings.data && !draft) setDraft(draftFrom(settings.data)) }, [settings.data, draft])
  const dirty = Boolean(draft && settings.data && JSON.stringify(draft) !== JSON.stringify(draftFrom(settings.data)))
  const update = useMutation({ mutationFn: api.updateDeliveryRates, onSuccess: next => {
    client.setQueryData(key, next); setDraft(draftFrom(next)); setError(''); setSuccess('Shipping destinations and prices saved.'); setQuote(null)
    void client.invalidateQueries({ queryKey: ['commercial-settings'] })
  }, onError: error => { setError(messageFor(error)); setSuccess('') } })
  const test = useMutation({ mutationFn: api.testDeliveryRate, onSuccess: result => { setQuote(result); setError('') }, onError: error => { setError(messageFor(error)); setQuote(null) } })

  function change(id: string, patch: Partial<DraftRate>) {
    setDraft(current => current && ({ ...current, rates: current.rates.map(rate => rate.id === id ? { ...rate, ...patch } : rate) }))
    setSuccess(''); setError(''); setQuote(null)
  }
  function save(event: FormEvent) {
    event.preventDefault()
    if (!draft) return
    setSuccess(''); setError('')
    try {
      const rates = draft.rates.map(({ amount, postal_prefixes, ...rate }) => {
        const amount_minor = minorAmount(amount)
        if (rate.active && amount_minor === null) throw new Error(`Enter a price for ${rate.region_name || 'each active destination'}, or deactivate that row.`)
        return { ...rate, region_name: rate.region_name.trim() || 'All regions', amount_minor, postal_prefixes: postal_prefixes.split(',').map(prefix => prefix.trim()).filter(Boolean) }
      })
      update.mutate({ enabled: draft.enabled, revision: draft.revision, rates, reason })
    } catch (error) { setError(error instanceof Error ? error.message : 'Check the shipping prices.') }
  }
  async function reload() {
    if (dirty && !window.confirm('Discard the unsaved changes and reload the saved table?')) return
    const result = await settings.refetch()
    if (result.data) { setDraft(draftFrom(result.data)); setError(''); setSuccess('Saved table reloaded.'); setQuote(null) }
  }

  const money = (amount: number, currency = settings.data?.currency ?? 'EUR') => new Intl.NumberFormat('en-GB', { style: 'currency', currency }).format(amount / 100)
  const activeCount = draft?.rates.filter(rate => rate.active && rate.amount.trim()).length ?? 0
  const countryCodes = [...(settings.data?.country_codes ?? [])].sort((a, b) => countryName(a).localeCompare(countryName(b)))

  return <section className="delivery-rates-section" aria-labelledby="delivery-rates-heading">
    <div className="section-heading"><div><p>Delivery configuration</p><h2 id="delivery-rates-heading">Shipping rates</h2></div><span>{activeCount} active destinations</span></div>
    {settings.isPending && <p role="status">Loading shipping destinations…</p>}
    {settings.isError && <p className="panel-error" role="alert">Shipping rates could not be loaded. <button type="button" onClick={() => void settings.refetch()}>Try again</button></p>}
    {draft && settings.data && <>
      <form className="delivery-rates-card" onSubmit={save}>
        <fieldset disabled={update.isPending}>
          <div className="delivery-rates-activation">
            <div className="delivery-rates-activation-icon"><Truck aria-hidden="true" /></div>
            <div><h3>One flat price per destination</h3><p>Prices apply once per order, independently of weight or quantity. All amounts are in {settings.data.currency}.</p></div>
            <label className="delivery-rates-toggle"><input type="checkbox" checked={draft.enabled} onChange={event => { setDraft({ ...draft, enabled: event.target.checked }); setSuccess(''); setQuote(null) }} /><span>Use these rates at checkout</span></label>
          </div>
          <p className={`delivery-rates-notice${draft.enabled ? ' enabled' : ''}`} role="status">{draft.enabled
            ? 'When saved, only destinations with an active, configured price can complete checkout. A specific postal rule takes priority, even when that destination is inactive.'
            : 'The table is off. Checkout keeps its current zero shipping charge until you configure prices, activate destinations and enable this table.'}</p>
          <p className="delivery-rates-intro">For a whole country, just choose the country and enter its price. Region names are optional. Use advanced configuration only when a region needs a different rate.</p>
          <div className="delivery-rates-table-scroll" role="region" aria-label="Editable shipping destinations" tabIndex={0}>
            <table className="delivery-rates-table"><caption className="sr-only">Edit shipping countries, optional regions, prices and availability</caption>
              <thead><tr><th scope="col">Country</th><th scope="col">Region <span className="delivery-rates-optional">(optional)</span></th><th scope="col">Price ({settings.data.currency})</th><th scope="col">Active</th><th scope="col"><span className="sr-only">Actions</span></th></tr></thead>
              <tbody>{draft.rates.map((rate, index) => <tr key={rate.id}>
                <td><label className="sr-only" htmlFor={`rate-country-${rate.id}`}>Country for destination {index + 1}</label><select id={`rate-country-${rate.id}`} value={rate.country_code} required onChange={event => change(rate.id, { country_code: event.target.value })}><option value="">Choose country</option>{countryCodes.map(code => <option key={code} value={code}>{countryName(code)}</option>)}</select></td>
                <td>
                  <label className="sr-only" htmlFor={`rate-region-${rate.id}`}>Region for destination {index + 1}</label><input id={`rate-region-${rate.id}`} value={rate.region_name} minLength={2} maxLength={100} placeholder="All regions" onChange={event => change(rate.id, { region_name: event.target.value })} />
                  <small className={rate.postal_prefixes.trim() ? 'delivery-rates-regional' : ''}>{rate.postal_prefixes.trim() ? 'Regional exception · detected automatically' : 'Entire country · no postal rules needed'}</small>
                  <details className="delivery-rates-advanced">
                    <summary>Advanced configuration<span className="sr-only"> for destination {index + 1}</span></summary>
                    <label htmlFor={`rate-postal-${rate.id}`}>Postal prefixes<span className="sr-only"> for destination {index + 1}</span></label>
                    <input id={`rate-postal-${rate.id}`} value={rate.postal_prefixes} maxLength={1200} placeholder="Leave empty for the whole country" onChange={event => change(rate.id, { postal_prefixes: event.target.value })} aria-describedby={`rate-postal-help-${rate.id}`} />
                    <p id={`rate-postal-help-${rate.id}`}>Only for regional exceptions. Enter starting characters separated by commas. Leaving this empty covers the whole country; the region name alone does not restrict coverage.</p>
                  </details>
                </td>
                <td><label className="sr-only" htmlFor={`rate-price-${rate.id}`}>Price for destination {index + 1}</label><input id={`rate-price-${rate.id}`} inputMode="decimal" value={rate.amount} placeholder="Not configured" maxLength={10} onChange={event => change(rate.id, { amount: event.target.value })} /><small>{rate.amount.trim() ? (Number(rate.amount.replace(',', '.')) === 0 ? 'Explicit free shipping' : 'Per order') : 'Enter a price to activate'}</small></td>
                <td><input type="checkbox" checked={rate.active} aria-label={`Activate destination ${index + 1}`} onChange={event => change(rate.id, { active: event.target.checked })} /></td>
                <td><button className="delivery-rates-remove" type="button" aria-label={`Remove destination ${index + 1}`} onClick={() => { setDraft({ ...draft, rates: draft.rates.filter(row => row.id !== rate.id) }); setSuccess(''); setQuote(null) }}><Trash2 size={17} aria-hidden="true" /></button></td>
              </tr>)}</tbody>
            </table>
            {!draft.rates.length && <p className="delivery-rates-empty">No destinations yet. Add a country or region below.</p>}
          </div>
          <div className="delivery-rates-table-footer"><button type="button" disabled={draft.rates.length >= 250} onClick={() => { setDraft({ ...draft, rates: [...draft.rates, { id: crypto.randomUUID(), country_code: '', region_name: 'All regions', postal_prefixes: '', amount: '', active: false }] }); setSuccess(''); setQuote(null) }}><Plus size={17} aria-hidden="true" /> Add destination</button><span>Blank price ≠ free shipping. Enter 0.00 explicitly for a free destination.</span></div>
          <details className="delivery-rates-help"><summary>How destinations are identified</summary><p>The country selected in the customer's delivery address determines the rate. You do not need postal rules for a country-wide price. For regional exceptions, the customer's postal code identifies the region automatically; a region name is only a label.</p><p>Portugal's mainland, Madeira and Azores rules are already configured. Keep those rules to preserve their separate prices. Open a row's advanced configuration only to change a regional exception. The most specific rule wins, including inactive rules; duplicate rules are rejected.</p></details>
          <div className="delivery-rates-save"><label>Reason for the change<input value={reason} onChange={event => setReason(event.target.value)} minLength={3} maxLength={500} required /></label><button className="primary-button" disabled={!dirty || update.isPending}><Save size={17} aria-hidden="true" />{update.isPending ? 'Saving…' : 'Save changes'}</button><button type="button" onClick={() => void reload()}>Reload saved table</button></div>
        </fieldset>
        {dirty && <p className="delivery-rates-draft" role="status">You have unsaved changes. Checkout still uses the last saved table.</p>}
        {success && <p className="delivery-rates-success" role="status">{success}</p>}
        {error && <p className="panel-error" role="alert">{error}</p>}
      </form>
      <section className="delivery-rates-test" aria-labelledby="delivery-test-heading">
        <div><span className="delivery-rates-test-icon"><MapPin aria-hidden="true" /></span><h3 id="delivery-test-heading">Test a destination</h3><p>Check the saved rules without creating an order. When the table is off, this is a simulation only.</p></div>
        <form onSubmit={event => { event.preventDefault(); setQuote(null); test.mutate({ country_code: testCountry, postal_code: testPostal }) }}>
          <label>Country<select aria-label="Country" value={testCountry} onChange={event => { setTestCountry(event.target.value); setQuote(null) }}>{countryCodes.map(code => <option key={code} value={code}>{countryName(code)}</option>)}</select></label>
          <label>Postal code<input aria-label="Postal code" value={testPostal} required maxLength={20} placeholder={testCountry === 'PT' ? '9000-001' : testCountry === 'ES' ? '28001' : 'Postal code'} onChange={event => { setTestPostal(event.target.value); setQuote(null) }} /></label>
          <button type="submit" disabled={dirty || update.isPending || test.isPending}>{test.isPending ? 'Checking…' : 'Test saved table'}</button>
        </form>
        {dirty && <p className="delivery-rates-draft">Save the changes above before testing them.</p>}
        {quote && <div className={`delivery-rates-test-result${quote.available ? ' available' : ''}`} role="status">{quote.available && quote.amount_minor != null ? <><strong>{money(quote.amount_minor, quote.currency)}</strong><span>{quote.region_name} · One charge per order{!quote.pricing_enabled ? ' · Simulation — table is off' : ''}</span></> : <><strong>{quote.issue_code === 'shipping_invalid_postal_code' ? 'Check the postal code' : 'Destination unavailable'}</strong><span>{quote.region_name ? `${quote.region_name}: no active price configured.` : 'No matching destination has been configured.'} Checkout will be blocked for this address when the table is on.</span></>}</div>}
      </section>
    </>}
  </section>
}
