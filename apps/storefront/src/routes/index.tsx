import { createFileRoute } from '@tanstack/react-router'
import {
  ArrowRight,
  ChevronRight,
  CircleUserRound,
  Heart,
  Menu,
  PackageCheck,
  Search,
  ShieldCheck,
  ShoppingBag,
  Sparkles,
} from 'lucide-react'

export const Route = createFileRoute('/')({
  component: HomePage,
})

const products = [
  { name: 'Ripple vase', price: '€34', tone: 'mauve', tag: 'Bestseller', form: 'vase' },
  { name: 'Soft-loop planter', price: '€42', tone: 'sand', tag: 'New', form: 'planter' },
  { name: 'Knot desk tray', price: '€28', tone: 'ink', tag: '', form: 'tray' },
  { name: 'Woven glow lamp', price: '€68', tone: 'clay', tag: 'Small batch', form: 'lamp' },
]

function IconButton({
  label,
  children,
}: Readonly<{ label: string; children: React.ReactNode }>) {
  return (
    <button className="icon-button" aria-label={label} type="button">
      {children}
    </button>
  )
}

function HomePage() {
  return (
    <>
      <div className="announcement">
        <Sparkles size={14} aria-hidden="true" />
        Small-batch objects, made slowly in Portugal
      </div>

      <header className="site-header">
        <a className="brand" href="/" aria-label="KnitPrint home">
          <img src="/logo.png" alt="KnitPrint" width="320" height="104" />
        </a>

        <nav className="desktop-nav" aria-label="Main navigation">
          <a href="#shop">Shop</a>
          <a href="#collections">Collections</a>
          <a href="#story">Our story</a>
        </nav>

        <div className="header-actions">
          <IconButton label="Search">
            <Search />
          </IconButton>
          <span className="desktop-action">
            <IconButton label="Account">
              <CircleUserRound />
            </IconButton>
          </span>
          <IconButton label="Cart, 0 items">
            <ShoppingBag />
          </IconButton>
          <span className="mobile-action">
            <IconButton label="Open menu">
              <Menu />
            </IconButton>
          </span>
        </div>
      </header>

      <main id="main-content">
        <section className="hero">
          <div className="hero-copy">
            <p className="eyebrow">Made between yarn and form</p>
            <h1>Soft ideas, shaped into lasting objects.</h1>
            <p className="hero-intro">
              Thoughtful homeware where the warmth of knitting meets the
              precision of 3D printing.
            </p>
            <div className="hero-actions">
              <a className="button button--primary" href="#shop">
                Explore the collection <ArrowRight size={18} />
              </a>
              <a className="text-link" href="#story">
                Discover our process
              </a>
            </div>
          </div>

          <div className="hero-art" aria-label="Sculptural KnitPrint objects">
            <div className="thread thread--one" />
            <div className="thread thread--two" />
            <div className="object object--tall">
              <span />
            </div>
            <div className="object object--round">
              <span />
            </div>
            <p>Layer by layer.<br />Loop by loop.</p>
          </div>
        </section>

        <section className="shop-section" id="shop">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Fresh from the studio</p>
              <h2>Objects with a softer edge</h2>
            </div>
            <a className="text-link desktop-action" href="/shop">
              Shop all pieces <ArrowRight size={17} />
            </a>
          </div>

          <div className="product-grid">
            {products.map((product) => (
              <article className="product-card" key={product.name}>
                <a href={`/products/${product.form}`} className={`product-image tone--${product.tone}`}>
                  {product.tag && <span className="product-tag">{product.tag}</span>}
                  <span className={`product-form product-form--${product.form}`} />
                  <button className="heart" aria-label={`Save ${product.name}`} type="button">
                    <Heart size={19} />
                  </button>
                </a>
                <div className="product-info">
                  <h3><a href={`/products/${product.form}`}>{product.name}</a></h3>
                  <p>{product.price}</p>
                </div>
              </article>
            ))}
          </div>
        </section>

        <section className="collections" id="collections">
          <article className="collection collection--home">
            <div>
              <p className="eyebrow">For your space</p>
              <h2>Quiet forms for everyday rituals</h2>
              <a className="circle-link" href="/collections/home" aria-label="Shop home collection">
                <ArrowRight />
              </a>
            </div>
          </article>
          <article className="collection collection--desk">
            <div>
              <p className="eyebrow">For your desk</p>
              <h2>Tactile tools for thoughtful work</h2>
              <a className="circle-link" href="/collections/desk" aria-label="Shop desk collection">
                <ArrowRight />
              </a>
            </div>
          </article>
        </section>

        <section className="story" id="story">
          <div className="story-mark" aria-hidden="true">
            <div className="yarn-ball" />
            <div className="story-thread" />
            <div className="printed-cube">KP</div>
          </div>
          <div className="story-copy">
            <p className="eyebrow">Two crafts, one point of view</p>
            <h2>Technology can still feel human.</h2>
            <p>
              We borrow the rhythm, softness, and patience of knitting, then
              build each object one precise layer at a time. The result is
              useful design with the warmth of something handmade.
            </p>
            <a className="text-link" href="/about">
              Meet KnitPrint <ArrowRight size={17} />
            </a>
          </div>
        </section>

        <section className="reassurance" aria-label="Shopping benefits">
          <div><PackageCheck /><span><strong>Made in small batches</strong><small>Less waste, more intention</small></span></div>
          <div><ShieldCheck /><span><strong>Secure checkout</strong><small>Your details stay protected</small></span></div>
          <div><Sparkles /><span><strong>Made to delight</strong><small>Designed and finished by hand</small></span></div>
        </section>
      </main>

      <footer className="site-footer">
        <div className="footer-lead">
          <img src="/logo.png" alt="KnitPrint" width="280" height="91" />
          <p>Objects with the soul of craft and the precision of print.</p>
        </div>
        <div className="footer-links">
          <div><h2>Shop</h2><a href="#shop">All pieces</a><a href="#collections">Collections</a><a href="/gift-cards">Gift cards</a></div>
          <div><h2>Help</h2><a href="/shipping">Shipping & returns</a><a href="/care">Care guide</a><a href="/contact">Contact</a></div>
          <div><h2>Follow along</h2><a href="/">Instagram</a><a href="/">Pinterest</a><a href="/">Studio notes</a></div>
        </div>
        <div className="footer-bottom">
          <span>© 2026 KnitPrint</span>
          <span>Made with care in Portugal</span>
          <a href="/privacy">Privacy</a>
          <a href="/terms">Terms</a>
        </div>
      </footer>
    </>
  )
}

