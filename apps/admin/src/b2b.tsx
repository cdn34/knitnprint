import { useEffect, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Building2, Download, FileText, Mail, Phone, Trash2 } from 'lucide-react'
import { ApiError, createApiClient, type B2bRequest } from '@knitnprint/api-client'

const api = createApiClient()
const statuses = { new: 'New', in_progress: 'In progress', completed: 'Completed' } as const
const productTypes: Record<string, string> = { clothing: 'Clothing', bottles: 'Bottles', backpacks: 'Backpacks', 'complete-kit': 'Complete kit', other: 'Other' }
const date = (value: string) => new Intl.DateTimeFormat('en-GB', { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(value))
const fileSize = (bytes: number) => bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(2)} MB` : bytes >= 1024 ? `${Math.ceil(bytes / 1024)} KB` : `${bytes} B`

export function B2bManagement({ initialRequestId, canManage }: Readonly<{ initialRequestId?: string; canManage: boolean }>) {
  const [page, setPage] = useState(0)
  const [deleted, setDeleted] = useState<{ id: string; reference: string } | null>(null)
  const selectedId = initialRequestId === deleted?.id ? undefined : initialRequestId
  const client = useQueryClient()
  const requests = useQuery({ queryKey: ['b2b', 'list', page], queryFn: () => api.listB2bRequests(page * 25), refetchInterval: 15_000 })
  const detail = useQuery({ queryKey: ['b2b', 'detail', selectedId], queryFn: () => api.b2bRequest(selectedId!), enabled: !!selectedId, refetchInterval: 10_000 })
  const update = useMutation({
    mutationFn: ({ id, status }: { id: string; status: string }) => api.updateB2bStatus(id, { status }),
    onSuccess: () => client.invalidateQueries({ queryKey: ['b2b'] }),
  })
  async function onDeleted(request: B2bRequest) {
    setDeleted({ id: request.id, reference: request.reference })
    window.location.hash = 'b2b'
    await Promise.all([
      client.cancelQueries({ queryKey: ['b2b', 'detail', request.id] }),
      client.cancelQueries({ queryKey: ['b2b', 'list'] }),
    ])
    client.removeQueries({ queryKey: ['b2b', 'detail', request.id] })
    client.setQueriesData<Array<B2bRequest>>({ queryKey: ['b2b', 'list'] }, (current) => current?.filter((item) => item.id !== request.id))
    if (page > 0 && requests.data?.length === 1) setPage((value) => value - 1)
    await client.invalidateQueries({ queryKey: ['b2b', 'list'] })
  }
  return (
    <section className="b2b-management" aria-labelledby="b2b-heading">
      <div className="section-heading">
        <div><p>Business · Proposals and corporate projects</p><h2 id="b2b-heading">B2B requests</h2></div>
        <span>Page {page + 1}</span>
      </div>
      {deleted && <p className="b2b-deleted-message" role="status">{deleted.reference} and its attachment were deleted.</p>}
      <div className="b2b-inbox">
        <aside className="b2b-request-list" aria-label="Business requests">
          {requests.isPending && <p className="panel-message">Loading requests…</p>}
          {requests.isError && <p className="panel-message error" role="alert">Requests could not be loaded. <button type="button" onClick={() => requests.refetch()}>Try again</button></p>}
          {requests.data?.length === 0 && <div className="b2b-empty"><Building2 aria-hidden="true" /><strong>No requests yet</strong><p>New business enquiries will appear here.</p></div>}
          {requests.data?.slice(0, 25).map((request) => (
            <a key={request.id} href={`#b2b/${request.id}`} className={request.id === selectedId ? 'selected' : ''} aria-current={request.id === selectedId ? 'page' : undefined}>
              <span><small>{request.reference}</small><B2bStatus value={request.status} /></span>
              <strong>{request.company}</strong>
              <span>{request.contact_name} · {request.quantity} pieces</span>
              <time dateTime={request.created_at}>{date(request.created_at)}</time>
            </a>
          ))}
          <div className="b2b-pagination">
            <button type="button" disabled={page === 0 || requests.isFetching} onClick={() => setPage((value) => value - 1)}>Previous</button>
            <button type="button" disabled={!requests.data || requests.data.length <= 25 || requests.isFetching} onClick={() => setPage((value) => value + 1)}>Next</button>
          </div>
        </aside>
        <div className="b2b-request-detail">
          {!selectedId && <div className="b2b-empty"><FileText aria-hidden="true" /><strong>Select a business request</strong><p>Review the project, contact details and saved attachment here.</p></div>}
          {selectedId && detail.isPending && <p className="panel-message">Loading request…</p>}
          {selectedId && detail.isError && <p className="panel-message error" role="alert">This request could not be loaded. <button type="button" onClick={() => detail.refetch()}>Try again</button></p>}
          {detail.data && selectedId && <>
            <header className="b2b-detail-heading">
              <div><small>{detail.data.reference}</small><h3>{detail.data.company}</h3><time dateTime={detail.data.created_at}>{date(detail.data.created_at)}</time></div>
              <B2bStatus value={detail.data.status} />
            </header>
            <dl className="b2b-contact-details">
              <div><dt>Contact</dt><dd>{detail.data.contact_name}</dd></div>
              <div><dt>Email</dt><dd><a href={`mailto:${detail.data.email}`}><Mail size={14} aria-hidden="true" />{detail.data.email}</a></dd></div>
              <div><dt>Phone</dt><dd><a href={`tel:${detail.data.phone}`}><Phone size={14} aria-hidden="true" />{detail.data.phone}</a></dd></div>
              <div><dt>Project</dt><dd>{productTypes[detail.data.product_type] ?? detail.data.product_type} · {detail.data.quantity} pieces</dd></div>
            </dl>
            <section className="b2b-detail-section"><h4>Project notes</h4><p className="b2b-notes">{detail.data.notes || 'No additional notes.'}</p></section>
            <Attachment request={detail.data} key={detail.data.id} />
            <section className="b2b-detail-section b2b-notification" aria-live="polite">
              <Mail size={18} aria-hidden="true" /><div><h4>Notification to support@knitnprint.com</h4>
                <p>{detail.data.notification_status === 'sent' ? `Sent · ${date(detail.data.notification_sent_at!)}`
                  : detail.data.notification_status === 'failed' ? 'Delivery failed after repeated attempts. The request and attachment remain saved.'
                  : detail.data.notification_attempts > 0 && detail.data.notification_status === 'pending' ? 'Delivery pending · an automatic retry is scheduled.' : 'Sending notification…'}</p>
              </div>
            </section>
            {canManage && <div className="b2b-status-control">
              <label htmlFor="b2b-request-status">Request status</label>
              <select id="b2b-request-status" value={detail.data.status} disabled={update.isPending} onChange={(event) => update.mutate({ id: detail.data.id, status: event.target.value })}>
                {Object.entries(statuses).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
              </select>
              {update.isError && <p role="alert">The status could not be saved. Please try again.</p>}
              {update.isSuccess && <p role="status">Status saved.</p>}
            </div>}
            {canManage && <DeleteRequest request={detail.data} key={detail.data.id} onDeleted={onDeleted} />}
          </>}
        </div>
      </div>
    </section>
  )
}

function DeleteRequest({ request, onDeleted }: Readonly<{ request: B2bRequest; onDeleted: (request: B2bRequest) => Promise<void> }>) {
  const [confirming, setConfirming] = useState(false)
  const opener = useRef<HTMLButtonElement>(null)
  const cancel = useRef<HTMLButtonElement>(null)
  const opened = useRef(false)
  useEffect(() => {
    if (confirming) {
      opened.current = true
      cancel.current?.focus()
    } else if (opened.current) {
      opener.current?.focus()
    }
  }, [confirming])
  const remove = useMutation({ mutationFn: () => api.deleteB2bRequest(request.id), onSuccess: () => onDeleted(request) })
  const sendingNotification = request.notification_status === 'processing'
  return <section className="b2b-detail-section b2b-delete-control" aria-labelledby="b2b-delete-heading">
    <h4 id="b2b-delete-heading">Delete request</h4>
    <p>Permanently remove this request, its original attachment and its image preview. This cannot be undone.</p>
    {confirming ? <div className="b2b-delete-confirmation" role="group" aria-label="Confirm request deletion" onKeyDown={(event) => {
      if (event.key === 'Escape' && !remove.isPending) { event.preventDefault(); setConfirming(false); remove.reset() }
    }}>
      <strong>Delete {request.reference} — {request.company}?</strong>
      <div>
        <button className="danger-action" type="button" disabled={remove.isPending || sendingNotification} onClick={() => remove.mutate()}>
          <Trash2 size={15} aria-hidden="true" />{remove.isPending ? 'Deleting…' : 'Permanently delete'}
        </button>
        <button type="button" ref={cancel} disabled={remove.isPending} onClick={() => { setConfirming(false); remove.reset() }}>Cancel</button>
      </div>
    </div> : <button className="danger-action" type="button" ref={opener} disabled={sendingNotification} onClick={() => setConfirming(true)}><Trash2 size={15} aria-hidden="true" />Delete request</button>}
    {sendingNotification && <p role="status">The support notification is being sent. Deletion will be available in a moment.</p>}
    {remove.isError && <p role="alert">{remove.error instanceof ApiError && remove.error.status === 409
      ? 'The support notification is being sent. Please try again in a moment.' : 'The request could not be deleted. Please try again.'}</p>}
  </section>
}

function B2bStatus({ value }: Readonly<{ value: string }>) {
  return <span className={`b2b-status b2b-status--${value}`}>{statuses[value as keyof typeof statuses] ?? value}</span>
}

function Attachment({ request }: Readonly<{ request: B2bRequest }>) {
  const [previewFailed, setPreviewFailed] = useState(false)
  const url = `/api/admin/b2b/requests/${encodeURIComponent(request.id)}/attachment`
  return <section className="b2b-detail-section b2b-attachment">
    <div className="b2b-attachment-heading"><div><h4>Saved attachment</h4><p>{request.attachment_name} · {fileSize(request.attachment_size)}</p></div>
      <a href={url} download={request.attachment_name}><Download size={16} aria-hidden="true" />Download original</a>
    </div>
    {request.has_preview && !previewFailed ? <div className="b2b-image-preview"><img src={`${url}?preview=true`} alt={`Attachment from ${request.company}: ${request.attachment_name}`} onError={() => setPreviewFailed(true)} /></div>
      : <p className="b2b-preview-unavailable">{previewFailed ? 'The preview could not be loaded. You can still download the original file.' : 'Download the original to view this document.'}</p>}
  </section>
}
