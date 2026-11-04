import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Product } from "@knitnprint/api-client";
type Value = {
  code: string;
  label: string;
  color?: string;
  media_ids?: string[];
};
type Axis = { code: string; label: string; control: string; values: Value[] };
type Config = { options: Axis[]; color_photos: boolean };
type Row = {
  id?: string;
  sku?: string;
  selections: Record<string, string>;
  price_minor: number;
  available_quantity: number;
  expected_quantity?: number;
  retired: boolean;
};
type Template = { id: string; definition: Axis; revision: number };
async function request<T>(
  path: string,
  body?: unknown,
  method = "PUT",
): Promise<T> {
  const response = await fetch(`/api/admin/${path}`, {
    credentials: "include",
    method: body ? method : "GET",
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await response.json();
  if (!response.ok) throw new Error(data.error?.message ?? "Could not save");
  return data;
}
const code = (label: string) =>
  label
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "")
    .slice(0, 6) || "OPTION";
const uniqueCode = (label: string, codes: string[]) => {
  const base = code(label);
  let next = base,
    n = 1;
  while (codes.includes(next)) next = base + n++;
  return next;
};
export function VariantMatrix({
  product,
  onSaved,
}: {
  product: Product;
  onSaved: () => void;
}) {
  const client = useQueryClient();
  const templates = useQuery({
    queryKey: ["variant-templates"],
    queryFn: () => request<Template[]>("variant-templates"),
  });
  const [config, setConfig] = useState<Config>(
    product.variant_config as unknown as Config,
  );
  const [rows, setRows] = useState<Row[]>(
    product.variants.map((v) => ({
      id: v.id,
      sku: v.sku,
      selections: (v.selections ?? {}) as Record<string, string>,
      price_minor: v.price_minor,
      available_quantity: v.available_quantity,
      expected_quantity: v.available_quantity,
      retired: v.retired,
    })),
  );
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [template, setTemplate] = useState<Template | null>(null);
  const changeAxis = (index: number, axis: Axis) =>
    setConfig((c) => ({
      ...c,
      options: c.options.map((o, i) => (i === index ? axis : o)),
    }));
  function generate() {
    let combos: Record<string, string>[] = [{}];
    for (const o of config.options) {
      if (combos.length * o.values.length > 1000) {
        setMessage("Too many combinations to preview (maximum 1,000).");
        return;
      }
      combos = combos.flatMap((s) =>
        o.values.map((v) => ({ ...s, [o.code]: v.code })),
      );
    }
    const key = (s: Record<string, string>) =>
      JSON.stringify(Object.entries(s).sort());
    const generated = combos.map(
      (selections) =>
        rows.find((r) => key(r.selections) === key(selections)) ?? {
          selections,
          price_minor: rows[0]?.price_minor ?? 0,
          available_quantity: 0,
          retired: false,
        },
    );
    setRows([
      ...generated,
      ...rows
        .filter((r) => r.id && !generated.some((g) => g.id === r.id))
        .map((r) => ({ ...r, retired: true })),
    ]);
    setMessage(
      "Preview ready. Review mappings, excluded combinations, prices and stock before saving.",
    );
  }
  async function save() {
    setBusy(true);
    setMessage("");
    try {
      const payload = {
        revision: product.variant_revision,
        config,
        variants: rows.filter((r) => r.id || !r.retired),
      };
      await request(
        `products/${product.id}/variant-matrix/preview`,
        payload,
        "POST",
      );
      await request(`products/${product.id}/variant-matrix`, payload);
      onSaved();
      setMessage("Variants saved.");
    } catch (e) {
      setMessage((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  const changeRow = (i: number, patch: Partial<Row>) =>
    setRows((rs) => rs.map((r, n) => (n === i ? { ...r, ...patch } : r)));
  return (
    <section className="catalog-editor variant-matrix">
      <h3>Options and variants</h3>
      <p>
        Product SKU: <strong>{product.sku}</strong> ·{" "}
        {rows.filter((r) => !r.retired).length}/100 active combinations
      </p>
      <p>
        Map existing variants before generating combinations. Templates are
        copied into this product. Labels and swatches can change; identity codes
        and saved SKUs stay permanent.
      </p>
      <label>
        Add option template{" "}
        <select
          value=""
          onChange={(e) => {
            const t = templates.data?.find((t) => t.id === e.target.value);
            if (t && !config.options.some((o) => o.code === t.definition.code))
              setConfig((c) => ({
                ...c,
                options: [...c.options, structuredClone(t.definition)],
              }));
          }}
        >
          <option value="">Choose template</option>
          {templates.data?.map((t) => (
            <option
              key={t.id}
              value={t.id}
              disabled={config.options.some(
                (o) => o.code === t.definition.code,
              )}
            >
              {t.definition.label} ({t.definition.control})
            </option>
          ))}
        </select>
      </label>
      {config.options.map((o, i) => (
        <fieldset key={o.code}>
          <legend>
            {o.label} · {o.code}
          </legend>
          <label>
            Option label
            <input
              value={o.label}
              onChange={(e) => changeAxis(i, { ...o, label: e.target.value })}
            />
          </label>
          <label>
            Display
            <select
              value={o.control}
              onChange={(e) => changeAxis(i, { ...o, control: e.target.value })}
            >
              <option value="buttons">Buttons</option>
              <option value="select">Dropdown</option>
              <option value="swatch">Color swatches</option>
            </select>
          </label>
          {o.values.map((v, j) => (
            <div className="option-value" key={v.code}>
              <small>{v.code}</small>
              <input
                aria-label={`${o.label} value ${j + 1}`}
                value={v.label}
                onChange={(e) =>
                  changeAxis(i, {
                    ...o,
                    values: o.values.map((v, n) =>
                      n === j ? { ...v, label: e.target.value } : v,
                    ),
                  })
                }
              />
              {o.control === "swatch" && (
                <input
                  aria-label={`Swatch ${v.label}`}
                  type="color"
                  value={v.color ?? "#000000"}
                  onChange={(e) =>
                    changeAxis(i, {
                      ...o,
                      values: o.values.map((v, n) =>
                        n === j ? { ...v, color: e.target.value } : v,
                      ),
                    })
                  }
                />
              )}
              <button
                type="button"
                onClick={() =>
                  changeAxis(i, {
                    ...o,
                    values: o.values.filter((_, n) => n !== j),
                  })
                }
              >
                Remove {v.label}
              </button>
              <button
                type="button"
                disabled={!j}
                onClick={() => {
                  const values = [...o.values];
                  [values[j - 1], values[j]] = [values[j], values[j - 1]];
                  changeAxis(i, { ...o, values });
                }}
              >
                Move up
              </button>
              {config.color_photos && o.control === "swatch" && (
                <fieldset>
                  <legend>
                    Photos for {v.label} (none uses shared gallery)
                  </legend>
                  {product.media.map((m) => (
                    <label key={m.id}>
                      <input
                        type="checkbox"
                        checked={v.media_ids?.includes(m.id) ?? false}
                        onChange={(e) =>
                          changeAxis(i, {
                            ...o,
                            values: o.values.map((v, n) =>
                              n === j
                                ? {
                                    ...v,
                                    media_ids: e.target.checked
                                      ? [...(v.media_ids ?? []), m.id]
                                      : (v.media_ids ?? []).filter(
                                          (id) => id !== m.id,
                                        ),
                                  }
                                : v,
                            ),
                          })
                        }
                      />
                      <img src={m.thumbnail_url} alt={m.alt_text} width={60} />
                    </label>
                  ))}
                </fieldset>
              )}
            </div>
          ))}
          <button
            type="button"
            onClick={() => {
              const label = window.prompt("New option value");
              if (label)
                changeAxis(i, {
                  ...o,
                  values: [
                    ...o.values,
                    {
                      code: uniqueCode(
                        label,
                        o.values.map((v) => v.code),
                      ),
                      label,
                      color: "#000000",
                    },
                  ],
                });
            }}
          >
            Add value
          </button>
          <button
            type="button"
            onClick={() =>
              setConfig((c) => ({
                ...c,
                options: c.options.filter((_, n) => n !== i),
              }))
            }
          >
            Remove option
          </button>
          <button
            type="button"
            disabled={i === 0}
            onClick={() =>
              setConfig((c) => {
                const options = [...c.options];
                [options[i - 1], options[i]] = [options[i], options[i - 1]];
                return { ...c, options };
              })
            }
          >
            Move option up
          </button>
        </fieldset>
      ))}
      <label>
        <input
          type="checkbox"
          checked={config.color_photos}
          onChange={(e) =>
            setConfig((c) => ({ ...c, color_photos: e.target.checked }))
          }
        />
        Use color-specific photos (optional)
      </label>
      <button type="button" onClick={generate}>
        Generate / refresh matrix preview
      </button>
      <label>
        Bulk price (€)
        <input
          type="number"
          min="0"
          step=".01"
          onChange={(e) => {
            if (e.target.value)
              setRows((rs) =>
                rs.map((r) =>
                  r.retired
                    ? r
                    : {
                        ...r,
                        price_minor: Math.round(Number(e.target.value) * 100),
                      },
                ),
              );
          }}
        />
      </label>
      <label>
        Bulk recorded stock
        <input
          type="number"
          step="1"
          onChange={(e) => {
            if (e.target.value)
              setRows((rs) =>
                rs.map((r) =>
                  r.retired
                    ? r
                    : { ...r, available_quantity: Number(e.target.value) },
                ),
              );
          }}
        />
      </label>
      <div className="matrix-scroll">
        <table>
          <thead>
            <tr>
              <th>Sell</th>
              {config.options.map((o) => (
                <th key={o.code}>{o.label}</th>
              ))}
              <th>Permanent SKU</th>
              <th>Price (€)</th>
              <th>Recorded stock</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              <tr key={r.id ?? JSON.stringify(r.selections)}>
                <td>
                  <input
                    aria-label={`Sell combination ${i + 1}`}
                    type="checkbox"
                    checked={!r.retired}
                    onChange={(e) =>
                      changeRow(i, { retired: !e.target.checked })
                    }
                  />
                </td>
                {config.options.map((o) => (
                  <td key={o.code}>
                    <select
                      aria-label={`Map ${o.label} ${i + 1}`}
                      disabled={Boolean(
                        product.variants.find((v) => v.id === r.id)?.selections,
                      )}
                      value={r.selections[o.code] ?? ""}
                      onChange={(e) =>
                        changeRow(i, {
                          selections: {
                            ...r.selections,
                            [o.code]: e.target.value,
                          },
                        })
                      }
                    >
                      <option value="">Choose</option>
                      {o.values.map((v) => (
                        <option key={v.code} value={v.code}>
                          {v.label}
                        </option>
                      ))}
                    </select>
                  </td>
                ))}
                <td>{r.sku ?? "Generated on save"}</td>
                <td>
                  <input
                    aria-label={`Price ${i + 1}`}
                    type="number"
                    min="0"
                    step=".01"
                    value={r.price_minor / 100}
                    onChange={(e) =>
                      changeRow(i, {
                        price_minor: Math.round(Number(e.target.value) * 100),
                      })
                    }
                  />
                </td>
                <td>
                  <input
                    aria-label={`Stock ${i + 1}`}
                    type="number"
                    step="1"
                    value={r.available_quantity}
                    onChange={(e) =>
                      changeRow(i, {
                        available_quantity: Number(e.target.value),
                      })
                    }
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <button
        type="button"
        disabled={busy || rows.filter((r) => !r.retired).length > 100}
        onClick={() => void save()}
      >
        Save options and variants
      </button>
      <p role="status">{message}</p>
      <details>
        <summary>Manage shared templates</summary>
        <p>Changes here affect future copies only.</p>
        {templates.data?.map((t) => (
          <button
            type="button"
            key={t.id}
            onClick={() => setTemplate(structuredClone(t))}
          >
            Edit {t.definition.label}
          </button>
        ))}
        <button
          type="button"
          onClick={() =>
            setTemplate({
              id: crypto.randomUUID(),
              revision: 0,
              definition: {
                code: "CUSTOM",
                label: "New option",
                control: "select",
                values: [{ code: "VALUE", label: "Value" }],
              },
            })
          }
        >
          New template
        </button>
        {template && (
          <TemplateEditor
            template={template}
            onChange={setTemplate}
            onSave={async () => {
              try {
                await request(`variant-templates/${template.id}`, template);
                await client.invalidateQueries({
                  queryKey: ["variant-templates"],
                });
                setTemplate(null);
                setMessage("Template saved.");
              } catch (e) {
                setMessage((e as Error).message);
              }
            }}
          />
        )}
      </details>
    </section>
  );
}
function TemplateEditor({
  template: t,
  onChange,
  onSave,
}: {
  template: Template;
  onChange: (t: Template) => void;
  onSave: () => void;
}) {
  const change = (definition: Axis) => onChange({ ...t, definition });
  return (
    <fieldset>
      <legend>Shared template</legend>
      <label>
        Name
        <input
          value={t.definition.label}
          onChange={(e) => change({ ...t.definition, label: e.target.value })}
        />
      </label>
      <label>
        Identity code
        <input
          maxLength={8}
          value={t.definition.code}
          onChange={(e) =>
            change({ ...t.definition, code: code(e.target.value) })
          }
        />
      </label>
      <label>
        Control
        <select
          value={t.definition.control}
          onChange={(e) => change({ ...t.definition, control: e.target.value })}
        >
          <option value="select">Dropdown</option>
          <option value="buttons">Buttons</option>
          <option value="swatch">Color swatches</option>
        </select>
      </label>
      {t.definition.values.map((v, i) => (
        <div key={i}>
          <input
            aria-label={`Default value ${i + 1}`}
            value={v.label}
            onChange={(e) =>
              change({
                ...t.definition,
                values: t.definition.values.map((v, n) =>
                  n === i ? { ...v, label: e.target.value } : v,
                ),
              })
            }
          />
          {t.definition.control === "swatch" && (
            <input
              type="color"
              aria-label={`Default color ${i + 1}`}
              value={v.color ?? "#000000"}
              onChange={(e) =>
                change({
                  ...t.definition,
                  values: t.definition.values.map((v, n) =>
                    n === i ? { ...v, color: e.target.value } : v,
                  ),
                })
              }
            />
          )}
          <button
            type="button"
            onClick={() =>
              change({
                ...t.definition,
                values: t.definition.values.filter((_, n) => i !== n),
              })
            }
          >
            Remove
          </button>
        </div>
      ))}
      <button
        type="button"
        onClick={() => {
          const label = window.prompt("New default value");
          if (label)
            change({
              ...t.definition,
              values: [
                ...t.definition.values,
                {
                  code: uniqueCode(
                    label,
                    t.definition.values.map((v) => v.code),
                  ),
                  label,
                  color: "#000000",
                },
              ],
            });
        }}
      >
        Add default
      </button>
      <button type="button" onClick={onSave}>
        Save template
      </button>
    </fieldset>
  );
}
