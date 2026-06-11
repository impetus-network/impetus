/* global React */
const { useState } = React;

function TopNav() {
  return (
    <nav style={{
      height: 64, background: 'var(--canvas)', display: 'flex', alignItems: 'center',
      padding: '0 32px', gap: 32, borderBottom: '1px solid var(--hairline-soft)',
      position: 'sticky', top: 0, zIndex: 10,
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <img src="../../assets/clay-mark.svg" width={28} height={28} alt="" />
        <span style={{ fontWeight: 600, fontSize: 18, letterSpacing: '-0.5px' }}>Clay</span>
      </div>
      <div style={{ display: 'flex', gap: 24 }}>
        {['Product','Solutions','Resources','Pricing','Customers'].map(l => (
          <a key={l} className="t-nav-link" style={{ textDecoration: 'none', cursor: 'pointer' }}>{l}</a>
        ))}
      </div>
      <div style={{ marginLeft: 'auto', display: 'flex', gap: 12, alignItems: 'center' }}>
        <button className="t-nav-link" style={{ background: 'none', border: 'none', cursor: 'pointer' }}>Sign in</button>
        <PrimaryButton>Try free</PrimaryButton>
      </div>
    </nav>
  );
}

function PrimaryButton({ children, onClick, size = 'md' }) {
  const pad = size === 'lg' ? '14px 24px' : '12px 20px';
  const h = size === 'lg' ? 48 : 44;
  return (
    <button onClick={onClick} className="t-button" style={{
      background: 'var(--primary)', color: 'var(--on-primary)', border: 'none', cursor: 'pointer',
      padding: pad, height: h, borderRadius: 'var(--radius-md)',
    }}>{children}</button>
  );
}

function SecondaryButton({ children, onClick }) {
  return (
    <button onClick={onClick} className="t-button" style={{
      background: 'var(--canvas)', color: 'var(--ink)',
      border: '1px solid var(--hairline)', cursor: 'pointer',
      padding: '12px 20px', height: 44, borderRadius: 'var(--radius-md)',
    }}>{children}</button>
  );
}

function Hero({ onCta }) {
  return (
    <section style={{
      maxWidth: 1280, margin: '0 auto', padding: '96px 32px',
      display: 'grid', gridTemplateColumns: '7fr 5fr', gap: 48, alignItems: 'center',
    }}>
      <div>
        <div className="t-caption-uppercase" style={{ color: 'var(--muted)', marginBottom: 24 }}>The GTM data platform</div>
        <h1 style={{ marginBottom: 24 }}>Go to market with unique data.</h1>
        <p style={{ fontSize: 20, lineHeight: 1.5, color: 'var(--body)', marginBottom: 32, maxWidth: 540 }}>
          Clay combines 100+ data providers, AI research agents, and the world's best go-to-market experts to turn ideas into pipeline.
        </p>
        <div style={{ display: 'flex', gap: 12 }}>
          <PrimaryButton size="lg" onClick={onCta}>Try free</PrimaryButton>
          <SecondaryButton>Book a demo</SecondaryButton>
        </div>
      </div>
      <div style={{
        background: 'var(--surface-soft)', borderRadius: 'var(--radius-xl)',
        aspectRatio: '4/3', overflow: 'hidden',
      }}>
        <img src="../../assets/illustrations/mountains.svg" alt="" style={{ width: '100%', height: '100%', objectFit: 'cover' }} />
      </div>
    </section>
  );
}

function FeatureCard({ variant, eyebrow, title, body, children }) {
  const palette = {
    pink:     { bg: 'var(--brand-pink)',    fg: 'var(--on-dark)' },
    teal:     { bg: 'var(--brand-teal)',    fg: 'var(--on-dark)' },
    lavender: { bg: 'var(--brand-lavender)',fg: 'var(--ink)' },
    peach:    { bg: 'var(--brand-peach)',   fg: 'var(--ink)' },
    ochre:    { bg: 'var(--brand-ochre)',   fg: 'var(--ink)' },
    cream:    { bg: 'var(--surface-card)',  fg: 'var(--ink)' },
  }[variant] || { bg: 'var(--surface-card)', fg: 'var(--ink)' };
  return (
    <div style={{
      background: palette.bg, color: palette.fg, padding: 32,
      borderRadius: 'var(--radius-xl)', display: 'flex', flexDirection: 'column', gap: 16,
      minHeight: 320,
    }}>
      <div className="t-caption-uppercase" style={{ opacity: 0.7 }}>{eyebrow}</div>
      <div style={{ fontFamily: 'var(--font-display)', fontWeight: 500, fontSize: 32, lineHeight: 1.15, letterSpacing: '-0.5px' }}>
        {title}
      </div>
      <p style={{ color: 'inherit', opacity: 0.85, fontSize: 15 }}>{body}</p>
      <div style={{ marginTop: 'auto' }}>{children}</div>
    </div>
  );
}

function ProductMockup({ rows }) {
  return (
    <div style={{
      background: 'var(--canvas)', borderRadius: 'var(--radius-md)',
      border: '1px solid rgba(0,0,0,0.08)', padding: 12, color: 'var(--ink)',
    }}>
      <div style={{ display: 'flex', gap: 6, marginBottom: 10 }}>
        {[0,1,2].map(i => <div key={i} style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--hairline)' }}/>)}
      </div>
      {rows.map((r, i) => (
        <div key={i} style={{
          display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          padding: '8px 4px', borderTop: i ? '1px solid var(--hairline-soft)' : 'none', fontSize: 12,
        }}>
          <span style={{ fontWeight: 500 }}>{r.label}</span>
          <span style={{ color: 'var(--muted)', fontFamily: 'var(--font-mono)' }}>{r.value}</span>
        </div>
      ))}
    </div>
  );
}

function FeatureGrid() {
  return (
    <section style={{ maxWidth: 1280, margin: '0 auto', padding: '0 32px 96px' }}>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 16 }}>
        <FeatureCard variant="pink" eyebrow="Outbound" title="Sequence with the data you actually have." body="Trigger campaigns from any signal — funding, hiring, tech stack.">
          <ProductMockup rows={[
            { label: 'Acme Corp', value: 'Series B' },
            { label: 'Globex', value: 'Hiring eng' },
            { label: 'Initech', value: 'Uses Stripe' },
          ]}/>
        </FeatureCard>
        <FeatureCard variant="teal" eyebrow="AI agents" title="Tell Claygent to do (almost) anything." body="Research prospects, qualify leads, personalize outreach — at scale.">
          <ProductMockup rows={[
            { label: 'Find decision-maker', value: '✓ done' },
            { label: 'Summarize 10-K', value: 'running' },
            { label: 'Draft intro', value: 'queued' },
          ]}/>
        </FeatureCard>
        <FeatureCard variant="lavender" eyebrow="Enrichment" title="100+ data providers in one row." body="Waterfall through providers automatically. Pay only for what you use.">
          <ProductMockup rows={[
            { label: 'Apollo', value: '✓ matched' },
            { label: 'Clearbit', value: '✓ matched' },
            { label: 'ZoomInfo', value: 'fallback' },
          ]}/>
        </FeatureCard>
      </div>
    </section>
  );
}

function PricingTier({ name, price, featured, features, cta }) {
  const bg = featured ? 'var(--brand-teal)' : 'var(--canvas)';
  const fg = featured ? 'var(--on-dark)' : 'var(--ink)';
  const border = featured ? 'none' : '1px solid var(--hairline)';
  return (
    <div style={{
      background: bg, color: fg, border, borderRadius: 'var(--radius-lg)',
      padding: 32, display: 'flex', flexDirection: 'column', gap: 16, minHeight: 380,
    }}>
      <div className="t-caption-uppercase" style={{ opacity: featured ? 0.7 : 1, color: featured ? 'var(--brand-mint)' : 'var(--muted)' }}>
        {featured ? 'Featured' : '\u00A0'}
      </div>
      <div className="t-title-lg">{name}</div>
      <div style={{ fontFamily: 'var(--font-display)', fontWeight: 500, fontSize: 48, letterSpacing: '-1.5px' }}>{price}</div>
      <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: 10, fontSize: 14 }}>
        {features.map(f => <li key={f} style={{ opacity: 0.9 }}>· {f}</li>)}
      </ul>
      <div style={{ marginTop: 'auto' }}>
        <button className="t-button" style={{
          width: '100%', height: 44, borderRadius: 'var(--radius-md)', cursor: 'pointer', border: 'none',
          background: featured ? 'var(--canvas)' : 'var(--primary)',
          color: featured ? 'var(--ink)' : 'var(--on-primary)',
        }}>{cta}</button>
      </div>
    </div>
  );
}

function PricingBand() {
  return (
    <section style={{ maxWidth: 1280, margin: '0 auto', padding: '96px 32px' }}>
      <div style={{ textAlign: 'center', marginBottom: 48 }}>
        <h2 style={{ marginBottom: 16 }}>Pricing for any team.</h2>
        <p style={{ fontSize: 18, color: 'var(--muted)' }}>Start free. Scale as you grow.</p>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 16 }}>
        <PricingTier name="Free" price="$0" features={['100 credits/mo','Core enrichment','Community support']} cta="Try free"/>
        <PricingTier name="Starter" price="$149" features={['2,000 credits/mo','All providers','Email support']} cta="Get started"/>
        <PricingTier name="Pro" price="$349" featured features={['10,000 credits/mo','Claygent included','Priority support','Webhooks']} cta="Start Pro trial"/>
        <PricingTier name="Enterprise" price="Custom" features={['Unlimited credits','SSO + audit logs','Dedicated CSM','SOC 2 Type II']} cta="Contact sales"/>
      </div>
    </section>
  );
}

function CTABand() {
  return (
    <section style={{ maxWidth: 1280, margin: '0 auto 96px', padding: '0 32px' }}>
      <div style={{
        background: 'var(--surface-soft)', borderRadius: 'var(--radius-xl)', padding: 80,
        display: 'grid', gridTemplateColumns: '2fr 1fr', alignItems: 'center', gap: 32,
      }}>
        <div>
          <h3 style={{ marginBottom: 16, maxWidth: 600 }}>Turn your growth ideas into reality today.</h3>
          <div style={{ display: 'flex', gap: 12, marginTop: 24 }}>
            <PrimaryButton size="lg">Try free</PrimaryButton>
            <SecondaryButton>Book a demo</SecondaryButton>
          </div>
        </div>
        <img src="../../assets/illustrations/mascot.svg" alt="" style={{ width: '100%', maxWidth: 280, justifySelf: 'end' }}/>
      </div>
    </section>
  );
}

function Footer() {
  const cols = [
    { h: 'Product', items: ['Overview','Claygent','Sequencer','Enrichment','Templates'] },
    { h: 'Solutions', items: ['Outbound','CRM hygiene','Inbound','Account research'] },
    { h: 'Resources', items: ['Docs','University','Community','Experts','Blog'] },
    { h: 'Company', items: ['About','Customers','Careers','Press','Contact'] },
  ];
  return (
    <footer style={{ background: 'var(--surface-soft)', padding: '80px 32px 48px' }}>
      <div style={{ maxWidth: 1280, margin: '0 auto', display: 'grid', gridTemplateColumns: '1.5fr repeat(4, 1fr)', gap: 32 }}>
        <div>
          <img src="../../assets/clay-logo.svg" width={120} height={38} alt=""/>
          <p style={{ fontSize: 14, color: 'var(--muted)', marginTop: 16, maxWidth: 240 }}>
            The GTM data platform.
          </p>
        </div>
        {cols.map(c => (
          <div key={c.h}>
            <div className="t-caption-uppercase" style={{ color: 'var(--muted)', marginBottom: 16 }}>{c.h}</div>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: 10 }}>
              {c.items.map(i => <li key={i} style={{ fontSize: 14, color: 'var(--body)', cursor: 'pointer' }}>{i}</li>)}
            </ul>
          </div>
        ))}
      </div>
      <div style={{ maxWidth: 1280, margin: '64px auto 0', borderTop: '1px solid var(--hairline)', paddingTop: 24, display: 'flex', justifyContent: 'space-between', fontSize: 13, color: 'var(--muted)' }}>
        <span>© 2026 Clay Labs, Inc.</span>
        <span>SOC 2 · GDPR · CCPA</span>
      </div>
    </footer>
  );
}

function SignupModal({ open, onClose }) {
  const [step, setStep] = useState(0);
  const [email, setEmail] = useState('');
  if (!open) return null;
  return (
    <div onClick={onClose} style={{
      position: 'fixed', inset: 0, background: 'rgba(10,10,10,0.4)',
      display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 100,
    }}>
      <div onClick={e => e.stopPropagation()} style={{
        background: 'var(--canvas)', borderRadius: 'var(--radius-xl)', padding: 40,
        width: 440, display: 'flex', flexDirection: 'column', gap: 16,
      }}>
        {step === 0 && (<>
          <h4 style={{ fontSize: 28, letterSpacing: '-0.5px' }}>Try Clay free.</h4>
          <p>No credit card required. 100 credits to start.</p>
          <input value={email} onChange={e => setEmail(e.target.value)} placeholder="you@company.com" style={{
            border: '1px solid var(--hairline)', borderRadius: 'var(--radius-md)', padding: '12px 16px',
            height: 44, fontSize: 16, fontFamily: 'var(--font-body)', outline: 'none',
          }}/>
          <PrimaryButton onClick={() => setStep(1)}>Continue</PrimaryButton>
        </>)}
        {step === 1 && (<>
          <h4 style={{ fontSize: 28, letterSpacing: '-0.5px' }}>Welcome aboard!</h4>
          <p>We sent a magic link to <strong>{email || 'your inbox'}</strong>.</p>
          <SecondaryButton onClick={onClose}>Close</SecondaryButton>
        </>)}
      </div>
    </div>
  );
}

Object.assign(window, {
  TopNav, PrimaryButton, SecondaryButton, Hero, FeatureCard, FeatureGrid,
  ProductMockup, PricingTier, PricingBand, CTABand, Footer, SignupModal,
});
