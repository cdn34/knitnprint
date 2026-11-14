import { useI18n } from "../i18n";

export function PasswordRequirements({ id }: Readonly<{ id: string }>) {
  const { t } = useI18n();
  return (
    <p id={id} className="account-password-requirements">
      {t("account.passwordRequirements")}
    </p>
  );
}
