import {
  ApiError,
  createApiClient,
  type CustomerAccountProfile,
} from "@knitprint/api-client";
import { createFileRoute } from "@tanstack/react-router";
import {
  ArrowLeft,
  CircleUserRound,
  Home,
  KeyRound,
  LogOut,
  MailCheck,
  MapPin,
} from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import {
  StorefrontAnnouncement,
  StorefrontFooter,
  StorefrontHeader,
} from "../components/storefront-shell";

export const Route = createFileRoute("/account")({
  component: AccountPage,
  head: () => ({
    meta: [
      { title: "Your account — KnitPrint" },
      {
        name: "description",
        content: "Manage your KnitPrint profile and delivery addresses.",
      },
    ],
  }),
});

const api = createApiClient();

type Mode = "login" | "register" | "forgot" | "reset";

function messageFor(error: unknown) {
  if (error instanceof ApiError) return error.body.error.message;
  return "Something went wrong. Please try again.";
}

function AccountPage() {
  const [profile, setProfile] = useState<CustomerAccountProfile | null>(null);
  const [checkingSession, setCheckingSession] = useState(true);
  const [mode, setMode] = useState<Mode>("login");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [resetToken, setResetToken] = useState("");

  useEffect(() => {
    let active = true;
    async function initialize() {
      const search = new URLSearchParams(window.location.search);
      const verificationToken = search.get("verify");
      const passwordToken = search.get("reset");
      if (verificationToken || passwordToken) {
        window.history.replaceState({}, "", window.location.pathname);
      }
      try {
        if (verificationToken) {
          await api.confirmCustomerVerification({ token: verificationToken });
          setNotice("Your email address is verified.");
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
          setError(messageFor(cause));
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
    try {
      const account =
        mode === "register"
          ? await api.registerCustomer({
              email: String(values.get("email") ?? ""),
              password: String(values.get("password") ?? ""),
              first_name: String(values.get("first_name") ?? ""),
              last_name: String(values.get("last_name") ?? ""),
              phone: String(values.get("phone") ?? "") || undefined,
            })
          : await api.loginCustomer({
              email: String(values.get("email") ?? ""),
              password: String(values.get("password") ?? ""),
            });
      setProfile(account);
      setNotice(
        mode === "register"
          ? "Your account is ready. Check your email to verify your address."
          : "Welcome back.",
      );
    } catch (cause) {
      setError(messageFor(cause));
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
        "If an account exists for that address, a password-reset email is on its way.",
      );
    } catch (cause) {
      setError(messageFor(cause));
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
      setNotice("Your password has been changed. Sign in with the new password.");
    } catch (cause) {
      setError(messageFor(cause));
    } finally {
      setPending(false);
    }
  }

  async function requestVerification() {
    setError("");
    setNotice("");
    setPending(true);
    try {
      await api.requestCustomerVerification();
      setNotice("Verification email requested. Please check your inbox.");
    } catch (cause) {
      setError(messageFor(cause));
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
    const values = new FormData(form);
    try {
      await api.addCustomerAddress({
        address_type: String(values.get("address_type") ?? "delivery"),
        recipient_name: String(values.get("recipient_name") ?? ""),
        line1: String(values.get("line1") ?? ""),
        line2: String(values.get("line2") ?? "") || undefined,
        city: String(values.get("city") ?? ""),
        region: String(values.get("region") ?? "") || undefined,
        postal_code: String(values.get("postal_code") ?? ""),
        country_code: String(values.get("country_code") ?? "").toUpperCase(),
        phone: String(values.get("address_phone") ?? "") || undefined,
      });
      setProfile(await api.customerAccount());
      form.reset();
      setNotice("Address saved.");
    } catch (cause) {
      setError(messageFor(cause));
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
      setNotice("You have signed out.");
    } catch (cause) {
      setError(messageFor(cause));
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
          <ArrowLeft size={17} aria-hidden="true" /> Back to the shop
        </a>
        <section className="account-intro" aria-labelledby="account-title">
          <div className="account-mark" aria-hidden="true">
            <CircleUserRound />
          </div>
          <p className="eyebrow">KnitPrint account</p>
          <h1 id="account-title">
            {profile
              ? `Welcome, ${profile.first_name}.`
              : "A home for your details."}
          </h1>
          <p>
            {profile
              ? "Keep your contact and delivery information ready for your next studio piece."
              : "Sign in or create an account to save delivery addresses. Guest checkout will always remain available."}
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
              <p>Finding your account…</p>
            </section>
          ) : profile ? (
            <AuthenticatedAccount
              profile={profile}
              pending={pending}
              onAddAddress={addAddress}
              onLogout={logout}
              onRequestVerification={requestVerification}
            />
          ) : (
            <GuestAccount
              mode={mode}
              pending={pending}
              onModeChange={(nextMode) => {
                setMode(nextMode);
                setError("");
                setNotice("");
              }}
              onSubmit={authenticate}
              onForgotPassword={forgotPassword}
              onResetPassword={resetPassword}
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
  onModeChange,
  onSubmit,
  onForgotPassword,
  onResetPassword,
}: Readonly<{
  mode: Mode;
  pending: boolean;
  onModeChange: (mode: Mode) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
  onForgotPassword: (event: FormEvent<HTMLFormElement>) => void;
  onResetPassword: (event: FormEvent<HTMLFormElement>) => void;
}>) {
  if (mode === "forgot" || mode === "reset") {
    return (
      <section className="account-panel" aria-labelledby="access-title">
        <div className="account-recovery-mark" aria-hidden="true">
          <KeyRound />
        </div>
        <div className="account-panel-heading">
          <p className="eyebrow">Secure account access</p>
          <h2 id="access-title">
            {mode === "forgot" ? "Reset your password" : "Choose a new password"}
          </h2>
          <p className="account-panel-copy">
            {mode === "forgot"
              ? "Enter your account email. We’ll send a one-hour reset link if the address is registered."
              : "Your new password must contain at least 12 characters."}
          </p>
        </div>
        <form
          className="account-form"
          onSubmit={mode === "forgot" ? onForgotPassword : onResetPassword}
        >
          {mode === "forgot" ? (
            <Field
              label="Email address"
              name="email"
              type="email"
              autoComplete="email"
            />
          ) : (
            <Field
              label="New password"
              name="password"
              type="password"
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
              ? "Please wait…"
              : mode === "forgot"
                ? "Send reset link"
                : "Change password"}
          </button>
          <button
            className="account-secondary-action"
            disabled={pending}
            type="button"
            onClick={() => onModeChange("login")}
          >
            Back to sign in
          </button>
        </form>
      </section>
    );
  }

  return (
    <section className="account-panel" aria-labelledby="access-title">
      <div className="account-tabs" role="group" aria-label="Account access">
        <button
          type="button"
          aria-pressed={mode === "login"}
          onClick={() => onModeChange("login")}
        >
          Sign in
        </button>
        <button
          type="button"
          aria-pressed={mode === "register"}
          onClick={() => onModeChange("register")}
        >
          Create account
        </button>
      </div>
      <div className="account-panel-heading">
        <p className="eyebrow">
          {mode === "login" ? "Welcome back" : "Join KnitPrint"}
        </p>
        <h2 id="access-title">
          {mode === "login" ? "Sign in to your account" : "Create your account"}
        </h2>
      </div>
      <form className="account-form" onSubmit={onSubmit}>
        {mode === "register" && (
          <div className="account-form-row">
            <Field
              label="First name"
              name="first_name"
              autoComplete="given-name"
            />
            <Field
              label="Last name"
              name="last_name"
              autoComplete="family-name"
            />
          </div>
        )}
        <Field
          label="Email address"
          name="email"
          type="email"
          autoComplete="email"
        />
        <Field
          label="Password"
          name="password"
          type="password"
          autoComplete={mode === "login" ? "current-password" : "new-password"}
          minLength={12}
        />
        {mode === "register" && (
          <Field
            label="Phone (optional)"
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
            ? "Please wait…"
            : mode === "login"
              ? "Sign in"
              : "Create account"}
        </button>
        {mode === "login" && (
          <button
            className="account-secondary-action"
            disabled={pending}
            type="button"
            onClick={() => onModeChange("forgot")}
          >
            Forgot your password?
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
  onLogout,
  onRequestVerification,
}: Readonly<{
  profile: CustomerAccountProfile;
  pending: boolean;
  onAddAddress: (event: FormEvent<HTMLFormElement>) => void;
  onLogout: () => void;
  onRequestVerification: () => void;
}>) {
  return (
    <div className="account-authenticated">
      <section
        className="account-panel account-profile"
        aria-labelledby="profile-title"
      >
        <div className="account-panel-heading account-panel-heading--action">
          <div>
            <p className="eyebrow">Contact details</p>
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
            <LogOut size={17} aria-hidden="true" /> Sign out
          </button>
        </div>
        <dl className="account-contact">
          <div>
            <dt>Email</dt>
            <dd>{profile.email}</dd>
          </div>
          {profile.phone && (
            <div>
              <dt>Phone</dt>
              <dd>{profile.phone}</dd>
            </div>
          )}
        </dl>
        <div
          className={`account-verification ${profile.email_verified ? "account-verification--complete" : ""}`}
        >
          <MailCheck aria-hidden="true" />
          <div>
            <strong>
              {profile.email_verified
                ? "Email address verified"
                : "Email verification needed"}
            </strong>
            <p>
              {profile.email_verified
                ? "Your account email is ready for secure order updates."
                : "Verify this address before using it for account recovery and future order updates."}
            </p>
          </div>
          {!profile.email_verified && (
            <button
              className="account-secondary-action"
              disabled={pending}
              type="button"
              onClick={onRequestVerification}
            >
              Send another link
            </button>
          )}
        </div>
      </section>

      <section
        className="account-panel account-addresses"
        aria-labelledby="addresses-title"
      >
        <div className="account-panel-heading">
          <p className="eyebrow">Saved places</p>
          <h2 id="addresses-title">Delivery addresses</h2>
        </div>
        {profile.addresses.length ? (
          <div className="address-grid">
            {profile.addresses.map((address) => (
              <article className="address-card" key={address.id}>
                <span className="address-kind">
                  <MapPin size={15} aria-hidden="true" /> {address.address_type}
                </span>
                <h3>{address.recipient_name}</h3>
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
                </address>
              </article>
            ))}
          </div>
        ) : (
          <div className="address-empty">
            <Home aria-hidden="true" />
            <p>No saved addresses yet.</p>
          </div>
        )}
      </section>

      <section className="account-panel" aria-labelledby="new-address-title">
        <div className="account-panel-heading">
          <p className="eyebrow">A new destination</p>
          <h2 id="new-address-title">Add an address</h2>
        </div>
        <form className="account-form" onSubmit={onAddAddress}>
          <label className="account-field">
            <span>Address type</span>
            <select name="address_type" defaultValue="delivery">
              <option value="delivery">Delivery</option>
              <option value="billing">Billing</option>
            </select>
          </label>
          <Field
            label="Recipient name"
            name="recipient_name"
            autoComplete="name"
          />
          <Field
            label="Address line 1"
            name="line1"
            autoComplete="address-line1"
          />
          <Field
            label="Address line 2 (optional)"
            name="line2"
            autoComplete="address-line2"
            required={false}
          />
          <div className="account-form-row account-form-row--location">
            <Field label="City" name="city" autoComplete="address-level2" />
            <Field
              label="Region (optional)"
              name="region"
              autoComplete="address-level1"
              required={false}
            />
          </div>
          <div className="account-form-row account-form-row--location">
            <Field
              label="Postal code"
              name="postal_code"
              autoComplete="postal-code"
            />
            <Field
              label="Country code"
              name="country_code"
              autoComplete="country"
              minLength={2}
              maxLength={2}
            />
          </div>
          <Field
            label="Delivery phone (optional)"
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
            {pending ? "Saving…" : "Save address"}
          </button>
        </form>
      </section>
    </div>
  );
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
}>) {
  return (
    <label className="account-field">
      <span>{label}</span>
      <input name={name} type={type} required={required} {...inputProps} />
    </label>
  );
}
