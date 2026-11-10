import { Check, KeyRound, Link2, ShieldCheck } from "lucide-react";
import { PasswordRequirements } from "./password-requirements";
import { useEffect, useRef, useState, type FormEvent } from "react";
import {
  createApiClient,
  type CustomerAccountProfile,
} from "@knitnprint/api-client";
import { useI18n } from "../i18n";
const api = createApiClient();
type Result = {
  status: string;
  profile: CustomerAccountProfile | null;
  google_authenticated_until?: number | null;
};
type GoogleApi = {
  initialize: (options: {
    client_id: string;
    nonce: string;
    callback: (response: { credential: string }) => void;
    auto_select: boolean;
  }) => void;
  renderButton: (
    element: HTMLElement,
    options: Record<string, unknown>,
  ) => void;
  revoke: (email: string, callback: () => void) => void;
  disableAutoSelect: () => void;
};
declare global {
  interface Window {
    google?: {
      accounts: {
        id: GoogleApi;
      };
    };
  }
}
let loading: Promise<void> | undefined;
function loadGoogle() {
  return (loading ??= new Promise<void>((resolve, reject) => {
    if (window.google) return resolve();
    const script = document.createElement("script");
    script.src = "https://accounts.google.com/gsi/client";
    script.async = true;
    script.onload = () => resolve();
    script.onerror = () => {
      loading = undefined;
      script.remove();
      reject(new Error("Google unavailable"));
    };
    document.head.append(script);
  }));
}
function GoogleButton({
  purpose,
  onResult,
  onChallenge,
  onError,
  onAvailability,
}: {
  purpose: string;
  onAvailability: (enabled: boolean) => void;
  onResult: (result: Result) => void;
  onChallenge: (id: string) => void;
  onError: (error: unknown) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const callbacks = useRef({ onResult, onChallenge, onError });
  callbacks.current = { onResult, onChallenge, onError };
  const { locale, t } = useI18n();
  useEffect(() => {
    let active = true;
    void (async () => {
      const config = await api.googleConfig();
      if (!config.client_id) return;
      onAvailability(true);
      await loadGoogle();
      if (!active) return;
      const challenge = await api.googleChallenge(purpose);
      if (!active || !ref.current) return;
      callbacks.current.onChallenge(challenge.id);
      window.google!.accounts.id.initialize({
        client_id: config.client_id,
        nonce: challenge.nonce,
        auto_select: false,
        callback: (response) => {
          void api
            .googleAuthenticate(challenge.id, response.credential)
            .then((result) => callbacks.current.onResult(result))
            .catch((error) => callbacks.current.onError(error));
        },
      });
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => resolve()),
      );
      if (!active || !ref.current) return;
      const width = Math.min(
        400,
        Math.floor(ref.current.getBoundingClientRect().width || 400),
      );
      ref.current.replaceChildren();
      window.google!.accounts.id.renderButton(ref.current, {
        theme: "outline",
        size: "large",
        shape: "pill",
        text: "continue_with",
        width,
        locale,
      });
    })().catch((error) => {
      if (active) callbacks.current.onError(error);
    });
    return () => {
      active = false;
    };
  }, [purpose, locale]);
  return (
    <div
      className="account-google-button"
      aria-label={t("account.google")}
      ref={ref}
    />
  );
}
export function GoogleAccess({
  onProfile,
  purpose = "sign_in",
  onReauthenticated,
}: {
  onProfile: (p: CustomerAccountProfile) => void;
  purpose?: string;
  onReauthenticated?: (until: number) => void;
}) {
  const { t } = useI18n();
  const [challenge, setChallenge] = useState("");
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const [resuming, setResuming] = useState(false);
  const [available, setAvailable] = useState(false);
  function received(result: Result) {
    setStatus(result.status);
    if (result.profile) onProfile(result.profile);
    const until =
      result.google_authenticated_until ??
      (result.status === "reauthenticated" ? Date.now() / 1000 + 300 : null);
    if (until && until * 1000 > Date.now()) onReauthenticated?.(until);
  }
  function failed(error: unknown) {
    setError(
      error instanceof Error ? error.message : t("account.genericError"),
    );
  }
  useEffect(() => {
    const search = new URLSearchParams(window.location.search);
    const token = search.get("google_confirm");
    const id = search.get("google_challenge");
    if (!token || !id) return;
    window.history.replaceState({}, "", window.location.pathname);
    setResuming(true);
    setChallenge(id);
    void api
      .googleConfirm({ challenge_id: id, token })
      .then(received)
      .catch(failed);
  }, []);
  async function confirm(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError("");
    const data = new FormData(event.currentTarget);
    try {
      received(
        await api.googleConfirm({
          challenge_id: challenge,
          password: String(data.get("password") || "") || undefined,
          first_name: String(data.get("first_name") || "") || undefined,
          last_name: String(data.get("last_name") || "") || undefined,
        }),
      );
    } catch (e) {
      failed(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div
      className={`account-google-access ${purpose === "sign_in" ? "account-google-access--entry" : "account-google-access--inline"}`}
      hidden={!available && !resuming && !status && !error}
    >
      {!status && !resuming && (
        <GoogleButton
          key={attempt}
          purpose={purpose}
          onAvailability={setAvailable}
          onResult={received}
          onChallenge={setChallenge}
          onError={failed}
        />
      )}
      {status === "reauthenticated" && (
        <>
          <p className="account-security-confirmed" role="status">
            <Check aria-hidden="true" />
            {t("account.identityConfirmed")}
          </p>
          <button
            className="account-security-secondary"
            type="button"
            onClick={() => {
              setStatus("");
              setAttempt((v) => v + 1);
            }}
          >
            {t("account.confirmAgain")}
          </button>
        </>
      )}
      {status === "confirmation_required" && (
        <>
          <p
            className="account-panel-copy account-google-instructions"
            role="status"
          >
            {t("account.googleConfirm")}
          </p>
          <form onSubmit={confirm} className="account-form">
            <label className="account-field">
              {t("account.currentPassword")}
              <input
                name="password"
                type="password"
                autoComplete="current-password"
                required
                maxLength={256}
              />
            </label>
            <button
              className="button button--primary account-security-primary"
              disabled={busy}
            >
              {t("account.confirm")}
            </button>
          </form>
        </>
      )}
      {status === "profile_required" && (
        <form onSubmit={confirm} className="account-form">
          <p className="account-panel-copy">{t("account.googleNames")}</p>
          <label className="account-field">
            {t("account.firstName")}
            <input
              name="first_name"
              required
              maxLength={100}
              autoComplete="given-name"
            />
          </label>
          <label className="account-field">
            {t("account.lastName")}
            <input
              name="last_name"
              required
              maxLength={100}
              autoComplete="family-name"
            />
          </label>
          <button
            className="button button--primary account-security-primary"
            disabled={busy}
          >
            {t("account.confirm")}
          </button>
        </form>
      )}
      {error && (
        <>
          <p className="account-message account-message--error" role="alert">
            {error}
          </p>
          <button
            className="account-security-secondary"
            type="button"
            onClick={() => {
              setError("");
              setStatus("");
              setResuming(false);
              setAttempt((v) => v + 1);
            }}
          >
            {t("account.tryAgain")}
          </button>
        </>
      )}
      {purpose === "sign_in" && (
        <div className="account-access-divider">
          <span>{t("account.continueWithEmail")}</span>
        </div>
      )}
    </div>
  );
}
export function SignInMethods({
  profile,
  onProfile,
  googleAuthenticatedUntil,
}: {
  profile: CustomerAccountProfile;
  onProfile: (p: CustomerAccountProfile) => void;
  googleAuthenticatedUntil?: number | null;
}) {
  const { t } = useI18n();
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [googleConfirmedUntil, setGoogleConfirmedUntil] = useState<
    number | null
  >(null);
  const [passwordConfirmed, setPasswordConfirmed] = useState(false);
  const googleConfirmed =
    googleConfirmedUntil !== null && googleConfirmedUntil * 1000 > Date.now();
  useEffect(() => {
    setGoogleConfirmedUntil(googleAuthenticatedUntil ?? null);
    if (googleAuthenticatedUntil) setPasswordConfirmed(false);
  }, [googleAuthenticatedUntil]);
  useEffect(() => {
    if (!googleConfirmedUntil) return;
    const timer = window.setTimeout(
      () => setGoogleConfirmedUntil(null),
      Math.max(0, googleConfirmedUntil * 1000 - Date.now()),
    );
    return () => window.clearTimeout(timer);
  }, [googleConfirmedUntil]);
  const methods = profile.auth_methods;
  async function run(
    work: () => Promise<void>,
    successMessage = t("account.methodsSaved"),
  ) {
    setError("");
    setNotice("");
    setBusy(true);
    try {
      await work();
      setNotice(successMessage);
    } catch (e) {
      setError(e instanceof Error ? e.message : t("account.genericError"));
    } finally {
      setBusy(false);
    }
  }
  async function confirmPassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const password = String(new FormData(form).get("password") || "");
    await run(async () => {
      await api.reauthenticateCustomer(password);
      setPasswordConfirmed(true);
      setGoogleConfirmedUntil(null);
      form.reset();
    }, t("account.identityConfirmed"));
  }
  async function savePassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    if (data.get("password") !== data.get("repeat")) {
      setError(t("account.passwordMismatch"));
      return;
    }
    await run(async () => {
      onProfile(await api.setCustomerPassword(String(data.get("password"))));
      setGoogleConfirmedUntil(null);
      setPasswordConfirmed(false);
      form.reset();
    });
  }
  async function remove(method: "google" | "password") {
    await run(async () => {
      const email = methods.google_email;
      onProfile(await api.removeCustomerMethod(method));
      setGoogleConfirmedUntil(null);
      setPasswordConfirmed(false);
      if (method === "google" && email && window.google) {
        window.google.accounts.id.disableAutoSelect();
        window.google.accounts.id.revoke(email, () => {});
      }
    });
  }
  return (
    <section
      className="account-panel account-security"
      aria-labelledby="account-security-title"
    >
      <div className="account-panel-heading account-security-heading">
        <div className="account-recovery-mark account-security-mark">
          <ShieldCheck aria-hidden="true" />
        </div>
        <div>
          <p className="eyebrow">{t("account.secureAccess")}</p>
          <h2 id="account-security-title">{t("account.signInMethods")}</h2>
          <p className="account-panel-copy">
            {t("account.signInMethodsIntro")}
          </p>
        </div>
      </div>

      <div className="account-security-methods">
        <div className="account-security-method">
          <div className="account-security-method-heading">
            <div className="account-security-method-icon">
              <Link2 aria-hidden="true" />
            </div>
            <div className="account-security-method-title">
              <h3>{t("account.google")}</h3>
              {methods.google_email && <p>{methods.google_email}</p>}
            </div>
            <span
              className={`account-method-status ${methods.google_email ? "account-method-status--connected" : ""}`}
            >
              {methods.google_email && <Check aria-hidden="true" />}
              {t(
                methods.google_email
                  ? "account.connected"
                  : "account.notConnected",
              )}
            </span>
          </div>
          <p className="account-security-help">
            {t(
              methods.google_email
                ? "account.googleMethodHelp"
                : "account.googleLinkHelp",
            )}
          </p>
          {methods.google_email &&
            (googleConfirmed ? (
              <p className="account-security-confirmed" role="status">
                <Check aria-hidden="true" />
                {t("account.identityConfirmed")}
              </p>
            ) : (
              <GoogleAccess
                key="reauthenticate"
                purpose="reauthenticate"
                onProfile={onProfile}
                onReauthenticated={(until) => {
                  setGoogleConfirmedUntil(until);
                  setPasswordConfirmed(false);
                }}
              />
            ))}
          {!methods.google_email && passwordConfirmed && (
            <GoogleAccess
              key="link"
              purpose="link"
              onProfile={(updated) => {
                setPasswordConfirmed(false);
                setGoogleConfirmedUntil(null);
                onProfile(updated);
              }}
              onReauthenticated={(until) => setGoogleConfirmedUntil(until)}
            />
          )}
          {methods.google_email && methods.password && (
            <div className="account-security-disconnect">
              <button
                className="account-security-secondary account-security-secondary--remove"
                type="button"
                disabled={busy || !passwordConfirmed}
                onClick={() => void remove("google")}
              >
                {t("account.removeGoogle")}
              </button>
              {!passwordConfirmed && (
                <p className="account-security-help">
                  {t("account.confirmPasswordToDisconnect")}
                </p>
              )}
            </div>
          )}
        </div>

        <div className="account-security-method">
          <div className="account-security-method-heading">
            <div className="account-security-method-icon">
              <KeyRound aria-hidden="true" />
            </div>
            <div className="account-security-method-title">
              <h3>{t("account.password")}</h3>
            </div>
            <span
              className={`account-method-status ${methods.password ? "account-method-status--connected" : ""}`}
            >
              {methods.password && <Check aria-hidden="true" />}
              {t(
                methods.password ? "account.connected" : "account.notConnected",
              )}
            </span>
          </div>
          <p className="account-security-help">
            {t("account.passwordMethodHelp")}
          </p>
          {methods.password && (
            <form
              className="account-form account-security-verification"
              onSubmit={confirmPassword}
            >
              <label className="account-field">
                {t("account.currentPassword")}
                <input
                  name="password"
                  type="password"
                  required
                  autoComplete="current-password"
                  maxLength={256}
                />
              </label>
              <button
                className="account-security-secondary"
                disabled={busy}
                type="submit"
              >
                {t("account.confirm")}
              </button>
            </form>
          )}
          <form
            className="account-form account-security-password-form"
            onSubmit={savePassword}
          >
            {!methods.password && !googleConfirmed && (
              <p className="account-security-help" role="status">
                {t("account.confirmGoogleForPassword")}
              </p>
            )}
            {methods.password && !passwordConfirmed && (
              <p className="account-security-help">
                {t("account.confirmPasswordForChange")}
              </p>
            )}
            <label className="account-field">
              {t("account.newPassword")}
              <input
                name="password"
                type="password"
                required
                minLength={12}
                maxLength={256}
                autoComplete="new-password"
                aria-describedby="profile-password-requirements"
              />
            </label>
            <PasswordRequirements id="profile-password-requirements" />
            <label className="account-field">
              {t("account.confirmPassword")}
              <input
                name="repeat"
                type="password"
                required
                minLength={12}
                maxLength={256}
                autoComplete="new-password"
              />
            </label>
            <button
              className="button button--primary account-security-primary"
              disabled={
                busy ||
                !(methods.password ? passwordConfirmed : googleConfirmed)
              }
            >
              {t(
                methods.password
                  ? "account.changePassword"
                  : "account.setPassword",
              )}
            </button>
          </form>
          {methods.google_email && methods.password && (
            <div className="account-security-disconnect">
              <button
                className="account-security-secondary account-security-secondary--remove"
                type="button"
                disabled={busy || !googleConfirmed}
                onClick={() => void remove("password")}
              >
                {t("account.removePassword")}
              </button>
              {!googleConfirmed && (
                <p className="account-security-help">
                  {t("account.confirmGoogleToRemovePassword")}
                </p>
              )}
            </div>
          )}
        </div>
      </div>
      {methods.google_email && (
        <p className="account-security-footnote">
          <ShieldCheck aria-hidden="true" />
          {t("account.googleEmailLocked")}
        </p>
      )}
      {notice && (
        <p
          className="account-message account-message--success account-security-feedback"
          role="status"
        >
          {notice}
        </p>
      )}
      {error && (
        <p
          className="account-message account-message--error account-security-feedback"
          role="alert"
        >
          {error}
        </p>
      )}
    </section>
  );
}
