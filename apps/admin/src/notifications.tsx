import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ApiError, createApiClient, type AdminNotification } from '@knitnprint/api-client'

const api = createApiClient()

function date(value: string) {
  return new Date(value).toLocaleString()
}

export function NotificationManagement({ canSend, canReadOrders }: Readonly<{
  canSend: boolean
  canReadOrders: boolean
}>) {
  const client = useQueryClient()
  const [filters, setFilters] = useState({ q: '', kind: '', status: 'outstanding', page: 1 })
  const [selected, setSelected] = useState<AdminNotification | null>(null)
  const notifications = useQuery({
    queryKey: ['notifications', filters],
    queryFn: () => api.listNotifications(filters),
    refetchInterval: 30_000,
  })
  const delivery = useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) => api.sendNotification(id, { reason }),
    onSettled: () => {
      client.invalidateQueries({ queryKey: ['notifications'] })
      client.invalidateQueries({ queryKey: ['order'] })
    },
    onSuccess: () => setSelected(null),
  })

  function search(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const form = new FormData(event.currentTarget)
    setFilters({
      q: String(form.get('q') ?? '').trim(),
      kind: String(form.get('kind') ?? '').trim(),
      status: String(form.get('status') ?? 'outstanding'),
      page: 1,
    })
    setSelected(null)
    delivery.reset()
  }

  function send(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!selected) return
    const reason = String(new FormData(event.currentTarget).get('reason') ?? '').trim()
    if (reason.length >= 3) delivery.mutate({ id: selected.id, reason })
  }

  return (
    <section className="notifications-section" aria-labelledby="notifications-heading">
      <div className="section-heading">
        <div>
          <p>Customer communications</p>
          <h2 id="notifications-heading">Email notifications</h2>
        </div>
        <button type="button" disabled={notifications.isFetching}
          onClick={() => client.invalidateQueries({ queryKey: ['notifications'] })}>Refresh</button>
      </div>
      <p>Find outstanding emails by order number, recipient, notification type, or job ID. Failed deliveries are retried by the hourly worker.</p>
      <form className="notification-filters" onSubmit={search}>
        <label>Search<input name="q" type="search" maxLength={320} placeholder="Order number or recipient email" /></label>
        <label>Status<select name="status" defaultValue="outstanding">
          <option value="outstanding">Outstanding</option>
          <option value="pending">Pending</option>
          <option value="failed">Failed</option>
          <option value="processing">Processing</option>
          <option value="sent">Sent</option>
          <option value="suppressed">Suppressed</option>
          <option value="all">All statuses</option>
        </select></label>
        <label>Notification type<input name="kind" maxLength={100} placeholder="Any type" /></label>
        <button type="submit">Search notifications</button>
      </form>
      {delivery.isSuccess && <p role="status" className={delivery.data.status === 'sent' ? 'panel-message' : 'panel-message error'}>
        {delivery.data.status === 'sent'
          ? `Email sent to ${delivery.data.recipient_email}.`
          : `Email could not be sent to ${delivery.data.recipient_email}. ${delivery.data.status === 'failed' ? 'Automatic retries are exhausted; another manual attempt is available.' : 'It remains pending for the worker to retry.'}`}
        {delivery.data.last_error && <span className="notification-error">{delivery.data.last_error}</span>}
      </p>}
      {delivery.isError && <p role="alert" className="panel-message error">
        {delivery.error instanceof ApiError ? delivery.error.body.error.message : 'The send request could not be completed. Refresh to check its status before trying again.'}
      </p>}
      {selected && canSend && <form className="notification-send" onSubmit={send}>
        <h3>Send email to {selected.recipient_email}</h3>
        <p>{selected.kind}{selected.order_number ? ` · ${selected.order_number}` : ''}. This attempts delivery immediately.</p>
        <label>Reason for sending<input name="reason" required minLength={3} maxLength={500} placeholder="Customer reported a missing confirmation" disabled={delivery.isPending} /></label>
        <button type="submit" disabled={delivery.isPending}>{delivery.isPending ? 'Sending…' : 'Send email now'}</button>
        <button type="button" disabled={delivery.isPending} onClick={() => setSelected(null)}>Cancel</button>
      </form>}
      {notifications.isPending && <p className="panel-message">Loading notifications…</p>}
      {notifications.isError && <p role="alert" className="panel-message error">Notifications could not be loaded. Try refreshing.</p>}
      {notifications.data && <>
        <p role="status">{notifications.data.total} notification{notifications.data.total === 1 ? '' : 's'} found</p>
        {notifications.data.items.length === 0 ? <p className="panel-message">No notifications match these filters.</p> :
          <div className="notification-table-wrap"><table className="notification-table">
            <thead><tr><th>Recipient / order</th><th>Type</th><th>Status</th><th>Attempts</th><th>Delivery details</th>{canSend && <th>Action</th>}</tr></thead>
            <tbody>{notifications.data.items.map((job) => <tr key={job.id}>
              <td><strong>{job.recipient_email}</strong>
                {job.order_number && (canReadOrders && job.order_id
                  ? <a href={`#orders/${job.order_id}`}>{job.order_number}</a>
                  : <span>{job.order_number}</span>)}
                <small>Job {job.id}</small>
              </td>
              <td>{job.kind.replaceAll('_', ' ')}</td>
              <td>{job.status}</td>
              <td>{job.attempt_count}</td>
              <td><span>Created {date(job.created_at)}</span>
                {job.sent_at && <span>Sent {date(job.sent_at)}</span>}
                {job.status === 'pending' && <span>Retry eligible {date(job.next_attempt_at)}</span>}
                {job.last_error && <span className="notification-error">{job.last_error}</span>}
              </td>
              {canSend && <td>{job.can_send
                ? <button type="button" aria-label={`Send ${job.kind.replaceAll('_', ' ')} to ${job.recipient_email}`}
                    disabled={delivery.isPending} onClick={() => { delivery.reset(); setSelected(job) }}>Send now</button>
                : <span>{job.status === 'sent' ? 'Sent' : job.status === 'suppressed' ? 'Suppressed' : job.status === 'processing' ? 'In progress' : 'Delivery unavailable'}</span>}
              </td>}
            </tr>)}</tbody>
          </table></div>}
        <nav className="notification-pagination" aria-label="Notification pages">
          <button type="button" disabled={filters.page <= 1 || notifications.isFetching || delivery.isPending}
            onClick={() => { setSelected(null); setFilters({ ...filters, page: filters.page - 1 }) }}>Previous</button>
          <span>Page {filters.page} of {Math.max(1, Math.ceil(notifications.data.total / notifications.data.page_size))}</span>
          <button type="button" disabled={filters.page * notifications.data.page_size >= notifications.data.total || notifications.isFetching || delivery.isPending}
            onClick={() => { setSelected(null); setFilters({ ...filters, page: filters.page + 1 }) }}>Next</button>
        </nav>
      </>}
    </section>
  )
}
