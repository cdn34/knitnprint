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
  const [enabled, setEnabled] = useState(false);
  useEffect(() => {
    let active = true;
    void (async () => {
      const config = await api.googleConfig();
      if (!config.client_id) return;
      setEnabled(true);
      onAvailability(true);
      await loadGoogle();
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
      ref.current.replaceChildren();
      window.google!.accounts.id.renderButton(ref.current, {
        theme: "outline",
        size: "large",
        locale,
      });
    })().catch((error) => {
      if (active) callbacks.current.onError(error);
    });
    return () => {
      active = false;
    };
  }, [purpose, locale]);
  return enabled ? <div aria-label={t("account.google")} ref={ref} /> : null;
}
export function GoogleAccess({
  onProfile,
  purpose = "sign_in",
  onReauthenticated,
}: {
  onProfile: (p: CustomerAccountProfile) => void;
  purpose?: string;
  onReauthenticated?: () => void;
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
    if (result.status === "reauthenticated") onReauthenticated?.();
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
      className="account-panel"
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
          <p role="status">{t("account.identityConfirmed")}</p>
          <button
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
          <p role="status">{t("account.googleConfirm")}</p>
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
            <button disabled={busy}>{t("account.confirm")}</button>
          </form>
        </>
      )}
      {status === "profile_required" && (
        <form onSubmit={confirm} className="account-form">
          <p>{t("account.googleNames")}</p>
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
          <button disabled={busy}>{t("account.confirm")}</button>
        </form>
      )}
      {error && (
        <>
          <p role="alert">{error}</p>
          <button
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
    </div>
  );
}
export function SignInMethods({
  profile,
  onProfile,
}: {
  profile: CustomerAccountProfile;
  onProfile: (p: CustomerAccountProfile) => void;
}) {
  const { t } = useI18n();
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [googleConfirmed, setGoogleConfirmed] = useState(false);
  const [passwordConfirmed, setPasswordConfirmed] = useState(false);
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
      setGoogleConfirmed(false);
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
      setGoogleConfirmed(false);
      setPasswordConfirmed(false);
      form.reset();
    });
  }
  async function remove(method: "google" | "password") {
    await run(async () => {
      const email = methods.google_email;
      onProfile(await api.removeCustomerMethod(method));
      setGoogleConfirmed(false);
      setPasswordConfirmed(false);
      if (method === "google" && email && window.google) {
        window.google.accounts.id.disableAutoSelect();
        window.google.accounts.id.revoke(email, () => {});
      }
    });
  }
  return (
    <section className="account-panel">
      <h2>{t("account.signInMethods")}</h2>
      <p>
        {t("account.password")}:{" "}
        {t(methods.password ? "account.connected" : "account.notConnected")}
      </p>
      <p>
        {t("account.google")}:{" "}
        {methods.google_email || t("account.notConnected")}
      </p>
      {methods.google_email && <p>{t("account.googleEmailLocked")}</p>}
      {methods.password && (
        <form className="account-form" onSubmit={confirmPassword}>
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
          <button disabled={busy}>{t("account.confirm")}</button>
        </form>
      )}
      {methods.google_email && (
        <GoogleAccess
          key="reauthenticate"
          purpose="reauthenticate"
          onProfile={onProfile}
          onReauthenticated={() => {
            setGoogleConfirmed(true);
            setPasswordConfirmed(false);
          }}
        />
      )}
      {!methods.google_email && passwordConfirmed && (
        <GoogleAccess key="link" purpose="link" onProfile={onProfile} />
      )}
      <form className="account-form" onSubmit={savePassword}>
        <label className="account-field">
          {t("account.newPassword")}
          <input
            name="password"
            type="password"
            required
            minLength={12}
            maxLength={256}
            autoComplete="new-password"
          />
        </label>
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
          disabled={
            busy || !(methods.password ? passwordConfirmed : googleConfirmed)
          }
        >
          {t(
            methods.password ? "account.changePassword" : "account.setPassword",
          )}
        </button>
      </form>
      {methods.google_email && methods.password && (
        <>
          <button
            type="button"
            disabled={busy || !passwordConfirmed}
            onClick={() => void remove("google")}
          >
            {t("account.removeGoogle")}
          </button>
          <button
            type="button"
            disabled={busy || !googleConfirmed}
            onClick={() => void remove("password")}
          >
            {t("account.removePassword")}
          </button>
        </>
      )}
      {notice && <p role="status">{notice}</p>}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
