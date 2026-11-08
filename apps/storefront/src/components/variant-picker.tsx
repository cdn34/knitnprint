import { useEffect, useState } from "react";
import type { Product, Variant } from "@knitnprint/api-client";
import { useI18n } from '../i18n';
export type Choice = {
  code: string;
  label: string;
  color?: string;
  media_ids?: string[];
};
export type Axis = {
  code: string;
  label: string;
  control: string;
  values: Choice[];
};
export type VariantConfiguration = { options: Axis[]; color_photos: boolean };
export function configuration(product: Product): VariantConfiguration {
  return (product.variant_config ?? {
    options: [],
    color_photos: false,
  }) as unknown as VariantConfiguration;
}
export function variantPhotos(product: Product, variant?: Variant) {
  const config = configuration(product);
  if (!config.color_photos) return product.media;
  const selections = variant?.selections as Record<string, string> | undefined;
  const ids = config.options
    .filter((o) => o.control === "swatch")
    .flatMap(
      (o) =>
        o.values.find((v) => v.code === selections?.[o.code])?.media_ids ?? [],
    );
  return ids.length
    ? product.media.filter((m) => ids.includes(m.id))
    : product.media;
}
export function variantPersonalization(product: Product, variant?: Variant) {
  const photos = variantPhotos(product, variant);
  if (photos === product.media || !Array.isArray(product.personalization.views))
    return product.personalization;
  return {
    ...product.personalization,
    views: product.personalization.views.filter(
      (v) =>
        v &&
        typeof v === "object" &&
        photos.some((p) => p.id === (v as { media_id?: string }).media_id),
    ),
  };
}
export function VariantPicker({
  product,
  variant,
  onChange,
}: {
  product: Product;
  variant?: Variant;
  onChange: (id: string) => void;
}) {
  const { t } = useI18n();
  const [hydrated, setHydrated] = useState(false);
  useEffect(() => setHydrated(true), []);
  const options = configuration(product).options;
  const selections = variant?.selections as Record<string, string> | undefined;
  if (!options.length) return null;
  return (
    <div className="structured-variant-picker">
      {options.map((axis, index) => ({ axis, index })).sort((a, b) => Number(b.axis.control === 'swatch') - Number(a.axis.control === 'swatch')).map(({ axis, index }) => {
        const match = (code: string) =>
          product.variants.find((v) => {
            const selected = v.selections as Record<string, string> | undefined;
            return (
              selected?.[axis.code] === code &&
              options.every(
                (o) =>
                  o.code === axis.code ||
                  selected?.[o.code] === selections?.[o.code],
              )
            );
          }) ??
          product.variants.find((v) => {
            const selected = v.selections as Record<string, string> | undefined;
            return (
              selected?.[axis.code] === code &&
              options
                .slice(0, index)
                .every((o) => selected?.[o.code] === selections?.[o.code])
            );
          });
        const choose = (code: string) => {
          const next = match(code);
          if (next) onChange(next.id);
        };
        const label = /^(color|colour)$/i.test(axis.label) ? t('product.variantColor')
          : /^size$/i.test(axis.label) ? t('product.variantSize')
          : /^size \(months\)$/i.test(axis.label) ? t('product.variantSizeMonths')
          : /^size \(years\)$/i.test(axis.label) ? t('product.variantSizeYears') : axis.label;
        return (
          <fieldset key={axis.code}>
            <legend>{label}</legend>
            {axis.control === "select" ? (
              <select
                disabled={!hydrated}
                aria-label={label}
                value={selections?.[axis.code] ?? ""}
                onChange={(e) => choose(e.target.value)}
              >
                {axis.values.map((v) => (
                  <option
                    key={v.code}
                    value={v.code}
                    disabled={!hydrated || !match(v.code)}
                  >
                    {v.label}
                  </option>
                ))}
              </select>
            ) : (
              <div className="structured-variant-options">
                {axis.values.map((v) => (
                  <button
                    type="button"
                    className="structured-variant-choice"
                    key={v.code}
                    aria-pressed={selections?.[axis.code] === v.code}
                    disabled={!hydrated || !match(v.code)}
                    onClick={() => choose(v.code)}
                  >
                    {axis.control === "swatch" && (
                      <span
                        className="color-swatch"
                        style={{ backgroundColor: v.color }}
                        aria-hidden="true"
                      />
                    )}
                    {v.label}
                  </button>
                ))}
              </div>
            )}
          </fieldset>
        );
      })}
    </div>
  );
}
