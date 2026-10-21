import {
  ApiError,
  createApiClient,
  type CustomerAccountProfile,
  type CreateAccountAddressRequest,
  type CustomerAddress,
  type CustomerOrderSummary,
} from "@knitnprint/api-client";
import { createFileRoute } from "@tanstack/react-router";
import {
  ArrowLeft,
  CircleUserRound,
  Eye,
  EyeOff,
  Home,
  KeyRound,
  LogOut,
  MailCheck,
  MapPin,
  PackageCheck,
  PackageOpen,
} from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import {
  StorefrontAnnouncement,
  StorefrontFooter,
  StorefrontHeader,
} from "../components/storefront-shell";
import { useI18n } from "../i18n";

export const Route = createFileRoute("/account")({
  component: AccountPage,
  head: () => ({
    meta: [
      { title: "Your account — KnitNPrint" },
      {
        name: "description",
        content: "Manage your KnitNPrint details, delivery addresses, and orders.",
      },
    ],
  }),
});

const api = createApiClient();

type Mode = "login" | "register" | "verify" | "forgot" | "reset";
type ProfileSaveFeedback = { kind: "success" | "error"; message: string };

function messageFor(error: unknown, fallback: string) {
  if (error instanceof ApiError) return error.body.error.message;
  return fallback;
}

function addressFromForm(values: FormData): CreateAccountAddressRequest {
  return {
    address_type: String(values.get("address_type") ?? "delivery"),
    recipient_name: String(values.get("recipient_name") ?? ""),
    line1: String(values.get("line1") ?? ""),
    line2: String(values.get("line2") ?? ""),
    city: String(values.get("city") ?? ""),
    region: String(values.get("region") ?? ""),
    postal_code: String(values.get("postal_code") ?? ""),
    country_code: String(values.get("country_code") ?? "").toUpperCase(),
    phone: String(values.get("address_phone") ?? ""),
  };
}

function AccountPage() {
  const { t } = useI18n();
  const [profile, setProfile] = useState<CustomerAccountProfile | null>(null);
  const [checkingSession, setCheckingSession] = useState(true);
  const [mode, setMode] = useState<Mode>("login");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [resetToken, setResetToken] = useState("");
  const [verificationEmail, setVerificationEmail] = useState("");

  useEffect(() => {
    let active = true;
    async function initialize() {
      const search = new URLSearchParams(window.location.search);
      const verificationToken = search.get("verify");
      const emailChangeToken = search.get("confirm_email");
      const passwordToken = search.get("reset");
      if (verificationToken || emailChangeToken || passwordToken) {
        window.history.replaceState({}, "", window.location.pathname);
      }
      try {
        if (verificationToken) {
          await api.confirmCustomerVerification({ token: verificationToken });
          setNotice(t("account.emailVerified"));
          setMode("login");
        }
        if (emailChangeToken) {
          await api.confirmCustomerEmailChange({ token: emailChangeToken });
          setNotice(t("account.emailChanged"));
        }
        if (passwordToken) {
          setResetToken(passwordToken);
          setMode("reset");
          return;
        }
        const account = await api.customerAccount();
        if (active) setProfile(account);
      } catch (cause) {
        if (active && !(cause instanceof ApiError && cause.status === 401)) {
          setError(messageFor(cause, t("account.genericError")));
        }
      } finally {
        if (active) setCheckingSession(false);
      }
    }
    void initialize();
    return () => {
      active = false;
    };
  }, []);

  async function authenticate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setNotice("");
    setPending(true);
    const values = new FormData(event.currentTarget);
    const email = String(values.get("email") ?? "").trim();
    const password = String(values.get("password") ?? "");
    if (mode === "register" && password !== String(values.get("confirm_password") ?? "")) {
      setError(t("account.passwordMismatch"));
      setPending(false);
      return;
    }
    try {
      const account =
        mode === "register"
          ? await api.registerCustomer({
              email,
              password,
              first_name: String(values.get("first_name") ?? ""),
              last_name: String(values.get("last_name") ?? ""),
              phone: String(values.get("phone") ?? "") || undefined,
            })
          : await api.loginCustomer({
              email,
              password,
            });
      if (mode === "register") {
        setVerificationEmail(account.email);
        setMode("verify");
      } else {
        setProfile(account);
      }
      setNotice(
        mode === "register"
          ? t("account.registered")
          : t("account.welcomeBack"),
      );
    } catch (cause) {
      if (cause instanceof ApiError && cause.body.error.code === "email_verification_required") {
        setVerificationEmail(email);
        setMode("verify");
      } else {
        setError(messageFor(cause, t("account.genericError")));
      }
    } finally {
      setPending(false);
    }
  }

  async function forgotPassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setNotice("");
    setPending(true);
    const values = new FormData(event.currentTarget);
    try {
      await api.forgotCustomerPassword({
        email: String(values.get("email") ?? ""),
      });
      setNotice(
        t("account.resetSent"),
      );
    } catch (cause) {
      setError(messageFor(cause, t("account.genericError")));
    } finally {
      setPending(false);
    }
  }

  async function resetPassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setNotice("");
    setPending(true);
    const values = new FormData(event.currentTarget);
    try {
      await api.resetCustomerPassword({
        token: resetToken,
        password: String(values.get("password") ?? ""),
      });
      setResetToken("");
      setMode("login");
      window.history.replaceState({}, "", window.location.pathname);
      setNotice(t("account.passwordChanged"));
    } catch (cause) {
      setError(messageFor(cause, t("account.genericError")));
    } finally {
      setPending(false);
    }
  }

  async function requestVerification(email: string) {
    setError("");
    setNotice("");
    setPending(true);
    try {
      await api.requestCustomerVerification(email);
      setNotice(t("account.verificationSent"));
    } catch (cause) {
      setError(messageFor(cause, t("account.genericError")));
    } finally {
      setPending(false);
    }
  }

  async function addAddress(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setNotice("");
    setPending(true);
    const form = event.currentTarget;
    try {
      await api.addCustomerAddress(addressFromForm(new FormData(form)));
      setProfile(await api.customerAccount());
      form.reset();
      setNotice(t("account.addressSaved"));
    } catch (cause) {
      setError(messageFor(cause, t("account.genericError")));
    } finally {
      setPending(false);
    }
  }

  async function updateAddress(addressId: string, event: FormEvent<HTMLFormElement>): Promise<boolean> {
    event.preventDefault();
    setError("");
    setNotice("");
    setPending(true);
    try {
      await api.updateCustomerAddress(addressId, addressFromForm(new FormData(event.currentTarget)));
      setProfile(await api.customerAccount());
      setNotice(t("account.addressUpdated"));
      return true;
    } catch (cause) {
      setError(messageFor(cause, t("account.genericError")));
      return false;
    } finally {
      setPending(false);
    }
  }

  async function updateProfile(event: FormEvent<HTMLFormElement>): Promise<ProfileSaveFeedback> {
    event.preventDefault();
    setError("");
    setNotice("");
    setPending(true);
    const values = new FormData(event.currentTarget);
    try {
      setProfile(await api.updateCustomerProfile({
        first_name: String(values.get("first_name") ?? ""),
        last_name: String(values.get("last_name") ?? ""),
        phone: String(values.get("phone") ?? ""),
      }));
      return { kind: "success", message: t("account.profileSaved") };
    } catch (cause) {
      return { kind: "error", message: messageFor(cause, t("account.genericError")) };
    } finally {
      setPending(false);
    }
  }

  async function requestEmailChange(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    setError("");
    setNotice("");
    setPending(true);
    const email = String(new FormData(form).get("new_email") ?? "").trim();
    try {
      await api.requestCustomerEmailChange({ email });
      setNotice(t("account.emailChangeSent", { email }));
      form.reset();
    } catch (cause) {
      setError(messageFor(cause, t("account.genericError")));
    } finally {
      setPending(false);
    }
  }

  async function logout() {
    setError("");
    setNotice("");
    setPending(true);
    try {
      await api.logoutCustomer();
      setProfile(null);
      setMode("login");
      setNotice(t("account.signedOut"));
    } catch (cause) {
      setError(messageFor(cause, t("account.genericError")));
    } finally {
      setPending(false);
    }
  }

  return (
    <>
      <StorefrontAnnouncement />
      <StorefrontHeader />

      <main className="account-page" id="main-content" tabIndex={-1}>
        <a className="text-link page-back-link" href="/">
          <ArrowLeft size={17} aria-hidden="true" /> {t("account.backToShop")}
        </a>
        <section className="account-intro" aria-labelledby="account-title">
          <div className="account-mark" aria-hidden="true">
            <CircleUserRound />
          </div>
          <p className="eyebrow">{t("account.eyebrow")}</p>
          <h1 id="account-title">
            {profile
              ? t("account.welcomeName", { name: profile.first_name })
              : t("account.guestTitle")}
          </h1>
          <p>
            {profile
              ? t("account.memberIntro")
              : t("account.guestIntro")}
          </p>
        </section>

        <div className="account-workspace">
          {error && (
            <p className="account-message account-message--error" role="alert">
              {error}
            </p>
          )}
          {notice && (
            <p
              className="account-message account-message--success"
              role="status"
            >
              {notice}
            </p>
          )}

          {checkingSession ? (
            <section
              className="account-panel account-loading"
              aria-live="polite"
            >
              <p>{t("account.finding")}</p>
            </section>
          ) : profile ? (
            <AuthenticatedAccount
              profile={profile}
              pending={pending}
              onAddAddress={addAddress}
              onEditAddress={updateAddress}
              onUpdateProfile={updateProfile}
              onRequestEmailChange={requestEmailChange}
              onLogout={logout}
              onRequestVerification={requestVerification}
            />
          ) : (
            <GuestAccount
              mode={mode}
              pending={pending}
              verificationEmail={verificationEmail}
              onModeChange={(nextMode) => {
                setMode(nextMode);
                setVerificationEmail("");
                setError("");
                setNotice("");
              }}
              onSubmit={authenticate}
              onForgotPassword={forgotPassword}
              onResetPassword={resetPassword}
              onRequestVerification={requestVerification}
            />
          )}
        </div>
      </main>
      <StorefrontFooter />
    </>
  );
}

function GuestAccount({
  mode,
  pending,
  verificationEmail,
  onModeChange,
  onSubmit,
  onForgotPassword,
  onResetPassword,
  onRequestVerification,
}: Readonly<{
  mode: Mode;
  pending: boolean;
  verificationEmail: string;
  onModeChange: (mode: Mode) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
  onForgotPassword: (event: FormEvent<HTMLFormElement>) => void;
  onResetPassword: (event: FormEvent<HTMLFormElement>) => void;
  onRequestVerification: (email: string) => void;
}>) {
  const { t } = useI18n();
  if (mode === "verify") {
    return (
      <section className="account-panel account-awaiting-verification" aria-labelledby="access-title">
        <div className="account-recovery-mark" aria-hidden="true"><MailCheck /></div>
        <div className="account-panel-heading">
          <p className="eyebrow">{t("account.secureAccess")}</p>
          <h2 id="access-title">{t("account.verifyToContinue")}</h2>
          <p className="account-panel-copy">{t("account.verifyInstructions", { email: verificationEmail })}</p>
        </div>
        <button className="button button--primary account-submit" disabled={pending} type="button" onClick={() => onRequestVerification(verificationEmail)}>
          {t(pending ? "account.pleaseWait" : "account.sendAnotherLink")}
        </button>
        <button className="account-secondary-action" type="button" onClick={() => onModeChange("login")}>{t("account.backToSignIn")}</button>
      </section>
    );
  }
  if (mode === "forgot" || mode === "reset") {
    return (
      <section className="account-panel" aria-labelledby="access-title">
        <div className="account-recovery-mark" aria-hidden="true">
          <KeyRound />
        </div>
        <div className="account-panel-heading">
          <p className="eyebrow">{t("account.secureAccess")}</p>
          <h2 id="access-title">
            {t(mode === "forgot" ? "account.resetPassword" : "account.choosePassword")}
          </h2>
          <p className="account-panel-copy">
            {mode === "forgot"
              ? t("account.forgotIntro")
              : t("account.newPasswordIntro")}
          </p>
        </div>
        <form
          className="account-form"
          onSubmit={mode === "forgot" ? onForgotPassword : onResetPassword}
        >
          {mode === "forgot" ? (
            <Field
              label={t("account.emailAddress")}
              name="email"
              type="email"
              autoComplete="email"
            />
          ) : (
            <PasswordField
              label={t("account.newPassword")}
              name="password"
              autoComplete="new-password"
              minLength={12}
            />
          )}
          <button
            className="button button--primary account-submit"
            disabled={pending}
            type="submit"
          >
            {pending
              ? t("account.pleaseWait")
              : mode === "forgot"
                ? t("account.sendReset")
                : t("account.changePassword")}
          </button>
          <button
            className="account-secondary-action"
            disabled={pending}
            type="button"
            onClick={() => onModeChange("login")}
          >
            {t("account.backToSignIn")}
          </button>
        </form>
      </section>
    );
  }

  return (
    <section className="account-panel" aria-labelledby="access-title">
      <div className="account-tabs" role="group" aria-label={t("account.accessLabel")}>
        <button
          type="button"
          aria-pressed={mode === "login"}
          onClick={() => onModeChange("login")}
        >
          {t("account.signIn")}
        </button>
        <button
          type="button"
          aria-pressed={mode === "register"}
          onClick={() => onModeChange("register")}
        >
          {t("account.createAccount")}
        </button>
      </div>
      <div className="account-panel-heading">
        <p className="eyebrow">
          {t(mode === "login" ? "account.welcomeBack" : "account.join")}
        </p>
        <h2 id="access-title">
          {t(mode === "login" ? "account.signInTitle" : "account.createTitle")}
        </h2>
      </div>
      <form className="account-form" onSubmit={onSubmit}>
        {mode === "register" && (
          <div className="account-form-row">
            <Field
              label={t("account.firstName")}
              name="first_name"
              autoComplete="given-name"
            />
            <Field
              label={t("account.lastName")}
              name="last_name"
              autoComplete="family-name"
            />
          </div>
        )}
        <Field
          label={t("account.emailAddress")}
          name="email"
          type="email"
          autoComplete="email"
        />
        <PasswordField
          label={t("account.password")}
          name="password"
          autoComplete={mode === "login" ? "current-password" : "new-password"}
          minLength={12}
        />
        {mode === "register" && (
          <PasswordField
            label={t("account.confirmPassword")}
            name="confirm_password"
            autoComplete="new-password"
            minLength={12}
          />
        )}
        {mode === "register" && (
          <Field
            label={t("account.phoneOptional")}
            name="phone"
            type="tel"
            autoComplete="tel"
            required={false}
          />
        )}
        <button
          className="button button--primary account-submit"
          disabled={pending}
          type="submit"
        >
          {pending
            ? t("account.pleaseWait")
            : mode === "login"
              ? t("account.signIn")
              : t("account.createAccount")}
        </button>
        {mode === "login" && (
          <button
            className="account-secondary-action"
            disabled={pending}
            type="button"
            onClick={() => onModeChange("forgot")}
          >
            {t("account.forgotPassword")}
          </button>
        )}
      </form>
    </section>
  );
}

function AuthenticatedAccount({
  profile,
  pending,
  onAddAddress,
  onEditAddress,
  onUpdateProfile,
  onRequestEmailChange,
  onLogout,
  onRequestVerification,
}: Readonly<{
  profile: CustomerAccountProfile;
  pending: boolean;
  onAddAddress: (event: FormEvent<HTMLFormElement>) => void;
  onEditAddress: (addressId: string, event: FormEvent<HTMLFormElement>) => Promise<boolean>;
  onUpdateProfile: (event: FormEvent<HTMLFormElement>) => Promise<ProfileSaveFeedback>;
  onRequestEmailChange: (event: FormEvent<HTMLFormElement>) => void;
  onLogout: () => void;
  onRequestVerification: (email: string) => void;
}>) {
  const { t } = useI18n();
  const [editingAddressId, setEditingAddressId] = useState<string | null>(null);
  const [profileFeedback, setProfileFeedback] = useState<ProfileSaveFeedback | null>(null);

  async function saveProfile(event: FormEvent<HTMLFormElement>) {
    setProfileFeedback(null);
    setProfileFeedback(await onUpdateProfile(event));
  }

  return (
    <div className="account-authenticated">
      <section
        className="account-panel account-profile"
        aria-labelledby="profile-title"
      >
        <div className="account-panel-heading account-panel-heading--action">
          <div>
            <p className="eyebrow">{t("account.contactDetails")}</p>
            <h2 id="profile-title">
              {profile.first_name} {profile.last_name}
            </h2>
          </div>
          <button
            className="account-logout"
            disabled={pending}
            type="button"
            onClick={onLogout}
          >
            <LogOut size={17} aria-hidden="true" /> {t("account.signOut")}
          </button>
        </div>
        <p className="account-current-email">{t("account.currentEmail")}: <strong>{profile.email}</strong></p>
        {profile.email_verified && <>
          <form className="account-form account-profile-form" onSubmit={saveProfile} onChange={() => setProfileFeedback(null)} key={`${profile.id}-${profile.first_name}-${profile.last_name}-${profile.phone}`}>
            <div className="account-form-row">
              <label className="account-field"><span>{t("account.firstName")}</span><input name="first_name" autoComplete="given-name" defaultValue={profile.first_name} required maxLength={100} /></label>
              <label className="account-field"><span>{t("account.lastName")}</span><input name="last_name" autoComplete="family-name" defaultValue={profile.last_name} required maxLength={100} /></label>
            </div>
            <label className="account-field"><span>{t("account.phoneOptional")}</span><input name="phone" type="tel" autoComplete="tel" defaultValue={profile.phone} maxLength={40} /></label>
            <button className="button button--primary account-profile-save" disabled={pending} type="submit">{t(pending ? "account.saving" : "account.saveProfile")}</button>
            {profileFeedback && <p className={`account-message account-profile-feedback account-message--${profileFeedback.kind}`} role={profileFeedback.kind === "error" ? "alert" : "status"}>{profileFeedback.message}</p>}
          </form>
          <form className="account-form account-email-form" onSubmit={onRequestEmailChange}>
            <div><h3>{t("account.changeEmail")}</h3><p>{t("account.changeEmailHelp")}</p></div>
            <div className="account-email-form-row">
              <label className="account-field"><span>{t("account.newEmail")}</span><input name="new_email" type="email" autoComplete="email" required /></label>
              <button className="account-email-submit" disabled={pending} type="submit">{t("account.sendConfirmation")}</button>
            </div>
          </form>
        </>}
        <div
          className={`account-verification ${profile.email_verified ? "account-verification--complete" : ""}`}
        >
          <MailCheck aria-hidden="true" />
          <div>
            <strong>
              {profile.email_verified
                ? t("account.emailVerifiedTitle")
                : t("account.emailVerificationNeeded")}
            </strong>
            <p>
              {profile.email_verified
                ? t("account.emailVerifiedBody")
                : t("account.emailVerificationBody")}
            </p>
          </div>
          {!profile.email_verified && (
            <button
              className="account-secondary-action"
              disabled={pending}
              type="button"
              onClick={() => onRequestVerification(profile.email)}
            >
              {t("account.sendAnotherLink")}
            </button>
          )}
        </div>
      </section>

      {profile.email_verified && <>
        <OrderSection kind="active" customerId={profile.id} />
        <OrderSection kind="history" customerId={profile.id} />
      </>}

      {profile.email_verified && <section
        className="account-panel account-addresses"
        aria-labelledby="addresses-title"
      >
        <div className="account-panel-heading">
          <p className="eyebrow">{t("account.savedPlaces")}</p>
          <h2 id="addresses-title">{t("account.deliveryAddresses")}</h2>
        </div>
        {profile.addresses.length ? (
          <div className="address-grid">
            {profile.addresses.map((address) => (
              <article className="address-card" key={address.id}>
                <div className="address-card-top"><span className="address-kind">
                  <MapPin size={15} aria-hidden="true" />{
                    address.address_type === "delivery"
                      ? t("account.delivery")
                      : address.address_type === "billing"
                        ? t("account.billing")
                        : address.address_type
                  }
                </span><button type="button" disabled={pending} onClick={() => setEditingAddressId(editingAddressId === address.id ? null : address.id)}>{t(editingAddressId === address.id ? "account.cancelEdit" : "account.editAddress")}</button></div>
                {editingAddressId === address.id ? <AddressEditForm address={address} pending={pending} onSubmit={async (event) => { if (await onEditAddress(address.id, event)) setEditingAddressId(null); }} /> : <><h3>{address.recipient_name}</h3>
                <address>
                  {address.line1}
                  <br />
                  {address.line2 && (
                    <>
                      {address.line2}
                      <br />
                    </>
                  )}
                  {address.postal_code} {address.city}
                  <br />
                  {address.region && (
                    <>
                      {address.region}
                      <br />
                    </>
                  )}
                  {address.country_code}
                </address></>}
              </article>
            ))}
          </div>
        ) : (
          <div className="address-empty">
            <Home aria-hidden="true" />
            <p>{t("account.noAddresses")}</p>
          </div>
        )}
      </section>}

      {profile.email_verified && <section className="account-panel" aria-labelledby="new-address-title">
        <div className="account-panel-heading">
          <p className="eyebrow">{t("account.newDestination")}</p>
          <h2 id="new-address-title">{t("account.addAddress")}</h2>
        </div>
        <form className="account-form" onSubmit={onAddAddress}>
          <label className="account-field">
            <span>{t("account.addressType")}</span>
            <select name="address_type" defaultValue="delivery">
              <option value="delivery">{t("account.delivery")}</option>
              <option value="billing">{t("account.billing")}</option>
            </select>
          </label>
          <Field
            label={t("account.recipientName")}
            name="recipient_name"
            autoComplete="name"
          />
          <Field
            label={t("account.addressLine1")}
            name="line1"
            autoComplete="address-line1"
          />
          <Field
            label={t("account.addressLine2")}
            name="line2"
            autoComplete="address-line2"
            required={false}
          />
          <div className="account-form-row account-form-row--location">
            <Field label={t("account.city")} name="city" autoComplete="address-level2" />
            <Field
              label={t("account.region")}
              name="region"
              autoComplete="address-level1"
              required={false}
            />
          </div>
          <div className="account-form-row account-form-row--location">
            <Field
              label={t("account.postalCode")}
              name="postal_code"
              autoComplete="postal-code"
            />
            <Field
              label={t("account.countryCode")}
              name="country_code"
              autoComplete="country"
              minLength={2}
              maxLength={2}
            />
          </div>
          <Field
            label={t("account.deliveryPhone")}
            name="address_phone"
            type="tel"
            autoComplete="tel"
            required={false}
          />
          <button
            className="button button--primary account-submit"
            disabled={pending}
            type="submit"
          >
            {t(pending ? "account.saving" : "account.saveAddress")}
          </button>
        </form>
      </section>}
    </div>
  );
}

function AddressEditForm({ address, pending, onSubmit }: Readonly<{ address: CustomerAddress; pending: boolean; onSubmit: (event: FormEvent<HTMLFormElement>) => void }>) {
  const { t } = useI18n();
  return <form className="account-form address-edit-form" onSubmit={onSubmit}>
    <label className="account-field"><span>{t("account.addressType")}</span><select name="address_type" defaultValue={address.address_type}><option value="delivery">{t("account.delivery")}</option><option value="billing">{t("account.billing")}</option></select></label>
    <Field label={t("account.recipientName")} name="recipient_name" defaultValue={address.recipient_name} />
    <Field label={t("account.addressLine1")} name="line1" defaultValue={address.line1} />
    <Field label={t("account.addressLine2")} name="line2" defaultValue={address.line2} required={false} />
    <Field label={t("account.city")} name="city" defaultValue={address.city} />
    <Field label={t("account.region")} name="region" defaultValue={address.region} required={false} />
    <div className="account-form-row account-form-row--location">
      <Field label={t("account.postalCode")} name="postal_code" defaultValue={address.postal_code} />
      <Field label={t("account.countryCode")} name="country_code" defaultValue={address.country_code} minLength={2} maxLength={2} />
    </div>
    <Field label={t("account.deliveryPhone")} name="address_phone" type="tel" defaultValue={address.phone} required={false} />
    <button className="button button--primary account-submit" disabled={pending} type="submit">{t(pending ? "account.saving" : "account.saveAddressChanges")}</button>
  </form>;
}

function OrderSection({ kind, customerId }: Readonly<{ kind: "active" | "history"; customerId: string }>) {
  const { t, locale, formatCurrency } = useI18n();
  const [orders, setOrders] = useState<CustomerOrderSummary[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    setOrders([]);
    setPage(0);
    setLoading(true);
    setError("");
    api.customerAccountOrders(kind).then((result) => {
      if (!active) return;
      setOrders(result.orders);
      setTotal(result.total);
      setPage(1);
    }).catch((cause) => {
      if (active) setError(messageFor(cause, t("account.ordersError")));
    }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [kind, customerId]);

  async function loadMore() {
    setLoading(true);
    setError("");
    try {
      const result = await api.customerAccountOrders(kind, page + 1);
      setOrders((current) => [...current, ...result.orders]);
      setTotal(result.total);
      setPage(result.page);
    } catch (cause) {
      setError(messageFor(cause, t("account.ordersError")));
    } finally {
      setLoading(false);
    }
  }

  const title = kind === "active" ? t("account.currentOrders") : t("account.orderHistory");
  const sectionId = `account-${kind}-orders`;
  return <section className="account-panel account-orders" aria-labelledby={sectionId}>
    <div className="account-panel-heading account-panel-heading--action">
      <div><p className="eyebrow">{t(kind === "active" ? "account.inProgress" : "account.pastPurchases")}</p><h2 id={sectionId}>{title}</h2></div>
      {total > 0 && <span className="account-order-count">{total}</span>}
    </div>
    {orders.length ? <div className="account-order-list">{orders.map((order) => <article className="account-order-card" key={order.id}>
      <div className="account-order-icon" aria-hidden="true">{kind === "active" ? <PackageOpen /> : <PackageCheck />}</div>
      <div className="account-order-body">
        <div className="account-order-top"><h3>{t("account.orderNumber", { number: order.order_number })}</h3><strong>{formatCurrency(order.total_minor, order.currency)}</strong></div>
        <p>{new Intl.DateTimeFormat(locale === "pt" ? "pt-PT" : locale === "es" ? "es-ES" : "en-GB", { dateStyle: "medium" }).format(new Date(order.created_at))} · {t("account.itemCount", { count: order.item_count })}</p>
        <div className="account-order-bottom"><span className={`account-order-status account-order-status--${order.order_status}`}>{t(`account.orderStatus.${order.order_status}` as Parameters<typeof t>[0])}</span><a href={`/cart?order_id=${encodeURIComponent(order.id)}&source=account`}>{t("account.viewOrder")}</a></div>
      </div>
    </article>)}</div> : !loading && !error && <div className="account-order-empty">{kind === "active" ? <PackageOpen aria-hidden="true" /> : <PackageCheck aria-hidden="true" />}<p>{t(kind === "active" ? "account.noCurrentOrders" : "account.noOrderHistory")}</p></div>}
    {error && <p className="account-message account-message--error" role="alert">{error}</p>}
    {loading && <p className="account-orders-loading" role="status">{t("account.loadingOrders")}</p>}
    {orders.length < total && !loading && <button className="account-order-more" type="button" onClick={() => void loadMore()}>{t("account.loadMore")}</button>}
    {error && !loading && !orders.length && <button className="account-order-more" type="button" onClick={() => void loadMore()}>{t("account.tryAgain")}</button>}
  </section>;
}

function Field({
  label,
  name,
  type = "text",
  required = true,
  ...inputProps
}: Readonly<{
  label: string;
  name: string;
  type?: string;
  required?: boolean;
  autoComplete?: string;
  minLength?: number;
  maxLength?: number;
  defaultValue?: string;
}>) {
  return (
    <label className="account-field">
      <span>{label}</span>
      <input name={name} type={type} required={required} {...inputProps} />
    </label>
  );
}

function PasswordField({
  label,
  name,
  autoComplete,
  minLength,
}: Readonly<{ label: string; name: string; autoComplete: string; minLength: number }>) {
  const { t } = useI18n();
  const [visible, setVisible] = useState(false);
  return (
    <div className="account-field">
      <label htmlFor={name}>{label}</label>
      <div className="account-password-control">
        <input id={name} name={name} type={visible ? "text" : "password"} autoComplete={autoComplete} minLength={minLength} required />
        <button
          type="button"
          aria-label={t(visible ? "account.hidePassword" : "account.showPassword", { field: label })}
          aria-pressed={visible}
          onClick={() => setVisible((current) => !current)}
        >
          {visible ? <EyeOff size={18} aria-hidden="true" /> : <Eye size={18} aria-hidden="true" />}
        </button>
      </div>
    </div>
  );
}
