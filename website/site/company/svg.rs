//! Architectural vector graphics, multi-facet brand marks, and 1px drafting diagrams for Kense.

/// Expanding-shaft action label matching the benchmark's directional link micro-interaction.
pub fn action_content(label: &str) -> String {
    format!(
        r#"<span class="kense-action-content"><span class="kense-action-label">{label}</span><svg class="kense-action-arrow" width="22" height="18" viewBox="0 0 24 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path class="kense-action-shaft" d="M3 10H19"/><path class="kense-action-tip" d="m3 6 4 4-4 4"/></svg></span>"#
    )
}

/// Sculpted multi-facet ribbon emblem for each Kense product badge, belt pill, and popover header.
pub fn product_emblem(kind: &str) -> String {
    crate::brand_marks::emblem(kind, 22)
}

/// CAD drafting guide lines and extrudable Kense geometric mark for the opening sequence.
pub fn intro_draft() -> &'static str {
    r#"<svg class="kense-intro-draft" viewBox="-120 -120 760 680" preserveAspectRatio="xMidYMid meet" fill="none" aria-hidden="true">
  <g class="kense-intro-guides">
    <path d="M-40 96H560" data-intro-trace pathLength="1"/>
    <path d="M-40 344H560" data-intro-trace pathLength="1"/>
    <path d="M124 -40V480" data-intro-trace pathLength="1"/>
    <path d="M204 -40V480" data-intro-trace pathLength="1"/>
    <path d="M164 256L428 -8" data-intro-trace pathLength="1"/>
    <path d="M164 184L428 448" data-intro-trace pathLength="1"/>
    <circle cx="260" cy="220" r="196" data-intro-trace pathLength="1"/>
    <circle cx="164" cy="220" r="42" data-intro-trace pathLength="1"/>
    <circle cx="364" cy="96" r="28" data-intro-trace pathLength="1"/>
    <circle cx="364" cy="344" r="28" data-intro-trace pathLength="1"/>
  </g>
  <g class="kense-intro-extrude-base">
    <path d="M124 72C124 60.954 132.954 52 144 52H184C195.046 52 204 60.954 204 72V368C204 379.046 195.046 388 184 388H144C132.954 388 124 379.046 124 368V72Z"/>
    <path d="M224 206L344 86C351.8 78.2 364.5 78.2 372.3 86L398 111.7C405.8 119.5 405.8 132.2 398 140L302 236H224V206Z"/>
    <path d="M298 214L400 316C407.8 323.8 407.8 336.5 400 344.3L374.3 370C366.5 377.8 353.8 377.8 346 370L224 248V214H298Z"/>
  </g>
  <g class="kense-intro-extrude-edges"></g>
  <g class="kense-intro-outline">
    <path d="M124 72C124 60.954 132.954 52 144 52H184C195.046 52 204 60.954 204 72V368C204 379.046 195.046 388 184 388H144C132.954 388 124 379.046 124 368V72Z" data-intro-trace pathLength="1"/>
    <path d="M224 206L344 86C351.8 78.2 364.5 78.2 372.3 86L398 111.7C405.8 119.5 405.8 132.2 398 140L302 236H224V206Z" data-intro-trace pathLength="1"/>
    <path d="M298 214L400 316C407.8 323.8 407.8 336.5 400 344.3L374.3 370C366.5 377.8 353.8 377.8 346 370L224 248V214H298Z" data-intro-trace pathLength="1"/>
  </g>
</svg>"#
}

/// Crisp industrial silhouette fallback inside `.kense-solid` before the 3D beveled-mesh renderer paints Frame 1.
pub fn solid_fallback(kind: &str) -> &'static str {
    match kind {
        "runtime" => {
            r##"<svg class="kense-solid-fallback" viewBox="0 0 440 330" fill="none" aria-hidden="true">
  <ellipse cx="226" cy="272" rx="148" ry="18" fill="currentColor" opacity="0.06"/>
  <rect x="118" y="68" width="228" height="156" rx="18" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
  <rect x="134" y="84" width="196" height="124" rx="11" fill="var(--surface-muted)" stroke="var(--border)" stroke-width="1"/>
  <rect x="64" y="122" width="76" height="48" rx="10" fill="#2563EB" opacity="0.9"/>
</svg>"##
        }
        "ui" => {
            r##"<svg class="kense-solid-fallback" viewBox="0 0 440 330" fill="none" aria-hidden="true">
  <ellipse cx="220" cy="276" rx="154" ry="18" fill="currentColor" opacity="0.06"/>
  <rect x="76" y="58" width="224" height="148" rx="14" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
  <path d="M58 210h260l20 34H38Z" fill="var(--surface-muted)" stroke="var(--border-strong)" stroke-width="1.2" stroke-linejoin="round"/>
  <rect x="284" y="98" width="84" height="162" rx="16" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
</svg>"##
        }
        "studio" => {
            r##"<svg class="kense-solid-fallback" viewBox="0 0 440 330" fill="none" aria-hidden="true">
  <ellipse cx="220" cy="276" rx="154" ry="18" fill="currentColor" opacity="0.06"/>
  <rect x="116" y="58" width="208" height="142" rx="14" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
  <rect x="52" y="152" width="68" height="58" rx="10" fill="var(--surface-muted)" stroke="var(--border)" stroke-width="1.1"/>
  <rect x="322" y="124" width="62" height="94" rx="10" fill="var(--surface-muted)" stroke="var(--border)" stroke-width="1.1"/>
</svg>"##
        }
        "klang" => {
            r##"<svg class="kense-solid-fallback" viewBox="0 0 440 330" fill="none" aria-hidden="true">
  <ellipse cx="224" cy="274" rx="152" ry="18" fill="currentColor" opacity="0.06"/>
  <rect x="112" y="64" width="236" height="156" rx="16" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
  <rect x="128" y="80" width="98" height="124" rx="10" fill="var(--surface-muted)" stroke="var(--border)" stroke-width="1"/>
  <rect x="238" y="80" width="94" height="36" rx="8" fill="var(--surface-muted)" stroke="var(--border)" stroke-width="1"/>
  <rect x="238" y="124" width="94" height="36" rx="8" fill="var(--surface-muted)" stroke="var(--border)" stroke-width="1"/>
  <rect x="238" y="168" width="94" height="36" rx="8" fill="var(--surface-muted)" stroke="var(--border)" stroke-width="1"/>
  <rect x="58" y="136" width="84" height="84" rx="16" fill="#E11D48" opacity="0.9"/>
</svg>"##
        }
        _ => {
            r##"<svg class="kense-solid-fallback" viewBox="0 0 440 330" fill="none" aria-hidden="true">
  <ellipse cx="220" cy="276" rx="152" ry="18" fill="currentColor" opacity="0.06"/>
  <rect x="96" y="74" width="84" height="148" rx="10" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
  <rect x="178" y="56" width="84" height="166" rx="10" fill="#2563EB" opacity="0.9"/>
  <rect x="260" y="82" width="84" height="140" rx="10" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
  <path d="M78 138h92l16-18h176v126a16 16 0 0 1-16 16H94a16 16 0 0 1-16-16Z" fill="var(--surface-muted)" stroke="var(--border-strong)" stroke-width="1.2"/>
</svg>"##
        }
    }
}

/// Orthographic 3D beveled mechanical miniature slot for `#capabilities`.
pub fn bento_relief(kind: &str) -> &'static str {
    match kind {
        "vault-chain" => {
            r##"<div class="kense-clay-stage" data-kense-solid="clay-vault" aria-hidden="true">
<svg class="kense-solid-fallback" viewBox="0 0 380 220" fill="none" aria-hidden="true" focusable="false">
  <ellipse cx="190" cy="194" rx="134" ry="12" fill="currentColor" opacity="0.055"/>
  <circle cx="96" cy="110" r="24" stroke="var(--border-strong)" stroke-width="8"/>
  <rect x="120" y="105" width="44" height="10" rx="4" fill="var(--border-strong)"/>
  <rect x="164" y="48" width="52" height="124" rx="14" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
  <rect x="228" y="74" width="76" height="72" rx="12" fill="#10B981" opacity="0.88"/>
</svg>
<canvas width="560" height="340"></canvas>
</div>"##
        }
        "fail-closed" => {
            r##"<div class="kense-clay-stage" data-kense-solid="clay-shield" aria-hidden="true">
<svg class="kense-solid-fallback" viewBox="0 0 320 220" fill="none" aria-hidden="true" focusable="false">
  <ellipse cx="160" cy="194" rx="96" ry="12" fill="currentColor" opacity="0.06"/>
  <path d="M126 92V66a34 34 0 0 1 68 0v26" stroke="var(--ink)" stroke-width="10" stroke-linecap="round"/>
  <rect x="96" y="86" width="128" height="94" rx="20" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
  <circle cx="160" cy="132" r="14" fill="#10B981"/>
</svg>
<canvas width="480" height="340"></canvas>
</div>"##
        }
        "release-primitive" => {
            r##"<div class="kense-clay-stage" data-kense-solid="clay-release" aria-hidden="true">
<svg class="kense-solid-fallback" viewBox="0 0 320 220" fill="none" aria-hidden="true" focusable="false">
  <ellipse cx="160" cy="194" rx="112" ry="12" fill="currentColor" opacity="0.055"/>
  <rect x="56" y="54" width="208" height="118" rx="18" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
  <rect x="78" y="72" width="44" height="44" rx="8" fill="var(--surface-muted)"/>
  <rect x="138" y="64" width="44" height="52" rx="8" fill="#2563EB"/>
  <rect x="198" y="72" width="44" height="44" rx="8" fill="#10B981"/>
</svg>
<canvas width="480" height="340"></canvas>
</div>"##
        }
        _ => {
            r##"<div class="kense-clay-stage" data-kense-solid="clay-rust" aria-hidden="true">
<svg class="kense-solid-fallback" viewBox="0 0 380 220" fill="none" aria-hidden="true" focusable="false">
  <ellipse cx="190" cy="194" rx="138" ry="12" fill="currentColor" opacity="0.055"/>
  <rect x="58" y="52" width="152" height="106" rx="14" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
  <rect x="228" y="56" width="90" height="122" rx="14" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="1.2"/>
</svg>
<canvas width="560" height="340"></canvas>
</div>"##
        }
    }
}

/// Animated topology connector buses for `#architecture`.
pub fn topology_connector(left: bool) -> &'static str {
    if left {
        r#"<svg class="kense-eco-connector" viewBox="0 0 64 480" preserveAspectRatio="none" aria-hidden="true" focusable="false">
  <path d="M2 80H18Q32 80 32 94V226Q32 240 46 240H62 M2 240H62 M2 400H18Q32 400 32 386V254Q32 240 46 240H62"/>
  <path class="kense-data-dot" d="M2 80H18Q32 80 32 94V226Q32 240 46 240H62" pathLength="100" style="--flow-delay:0s"/>
  <path class="kense-data-dot" d="M2 240H62" pathLength="100" style="--flow-delay:3.2s"/>
  <path class="kense-data-dot" d="M2 400H18Q32 400 32 386V254Q32 240 46 240H62" pathLength="100" style="--flow-delay:6.4s"/>
  <circle cx="2" cy="80" r="1.8"/><circle cx="2" cy="240" r="1.8"/><circle cx="2" cy="400" r="1.8"/><circle cx="62" cy="240" r="1.8"/>
</svg>"#
    } else {
        r#"<svg class="kense-eco-connector" viewBox="0 0 64 480" preserveAspectRatio="none" aria-hidden="true" focusable="false">
  <path d="M2 240H18Q32 240 32 226V94Q32 80 46 80H62 M2 240H62 M18 240Q32 240 32 254V386Q32 400 46 400H62"/>
  <path class="kense-data-dot" d="M2 240H18Q32 240 32 226V94Q32 80 46 80H62" pathLength="100" style="--flow-delay:1.6s"/>
  <path class="kense-data-dot" d="M2 240H62" pathLength="100" style="--flow-delay:4.8s"/>
  <path class="kense-data-dot" d="M2 240H18Q32 240 32 254V386Q32 400 46 400H62" pathLength="100" style="--flow-delay:8.0s"/>
  <circle cx="2" cy="240" r="1.8"/><circle cx="62" cy="80" r="1.8"/><circle cx="62" cy="240" r="1.8"/><circle cx="62" cy="400" r="1.8"/>
</svg>"#
    }
}

/// Precision `1px` architectural UI drafting illustrations inside the 3 fanned `#start` cards.
pub fn access_graphic(kind: &str) -> &'static str {
    match kind {
        "ui" => {
            r#"<div class="kense-access-art" data-kind="ui" aria-hidden="true"><div class="kense-access-viewport">
<svg viewBox="0 0 400 268" width="340" height="228" preserveAspectRatio="xMaxYMid meet" overflow="visible" fill="none">
  <rect x="28" y="18" width="344" height="226" rx="12" fill="var(--art-hi)" stroke="var(--art-mid)" stroke-width="1"/>
  <path d="M40 18H360Q372 18 372 30V44H28V30Q28 18 40 18Z" fill="var(--art-soft)"/>
  <line x1="28" y1="44" x2="372" y2="44" stroke="var(--art-mid)" stroke-width="1"/>
  <circle cx="44" cy="31" r="3" fill="var(--art-deep)"/><circle cx="55" cy="31" r="3" fill="var(--art-deep)"/><circle cx="66" cy="31" r="3" fill="var(--art-deep)"/>
  <rect x="164" y="28.5" width="72" height="5" rx="2.5" fill="var(--art-mid)"/>
  <line x1="118" y1="44" x2="118" y2="244" stroke="var(--art-mid)" stroke-width="1"/>
  <rect x="42" y="60" width="46" height="5" rx="2.5" fill="var(--art-deep)"/>
  <rect x="42" y="78" width="60" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <rect x="42" y="94" width="52" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <rect x="42" y="110" width="58" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <rect x="42" y="126" width="44" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <rect x="138" y="62" width="212" height="56" rx="9" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  <rect x="152" y="76" width="84" height="6" rx="3" fill="var(--art-deep)"/>
  <rect x="152" y="90" width="132" height="5" rx="2.5" fill="var(--art-mid)"/>
  <rect x="138" y="132" width="100" height="68" rx="9" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  <rect x="250" y="132" width="100" height="68" rx="9" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  <g class="kense-access-moving">
    <rect x="206" y="96" width="174" height="96" rx="12" fill="var(--art-hi)" stroke="var(--art-edge)" stroke-width="1.2"/>
    <circle cx="230" cy="122" r="11" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
    <path d="m226 122 2.8 2.8 5.4-5.6" stroke="var(--art-edge)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
    <rect x="250" y="115" width="76" height="6" rx="3" fill="var(--art-edge)"/>
    <rect x="250" y="127" width="52" height="4.5" rx="2.25" fill="var(--art-deep)"/>
    <rect x="222" y="152" width="142" height="24" rx="12" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
    <rect x="225" y="155" width="66" height="18" rx="9" fill="var(--art-hi)" stroke="var(--art-mid)" stroke-width="1"/>
  </g>
</svg></div></div>"#
        }
        "runtime" => {
            r#"<div class="kense-access-art" data-kind="runtime" aria-hidden="true"><div class="kense-access-viewport">
<svg viewBox="0 0 400 268" width="340" height="228" preserveAspectRatio="xMaxYMid meet" overflow="visible" fill="none">
  <line x1="68" y1="134" x2="344" y2="134" stroke="var(--art-mid)" stroke-width="1.2" stroke-dasharray="4 4"/>
  <rect x="34" y="28" width="84" height="212" rx="12" fill="var(--art-hi)" stroke="var(--art-mid)" stroke-width="1"/>
  <circle cx="76" cy="64" r="13" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  <path d="m71.5 64 3 3 6-6.5" stroke="var(--art-edge)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
  <rect x="52" y="96" width="48" height="5.5" rx="2.75" fill="var(--art-deep)"/>
  <rect x="52" y="112" width="36" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <rect x="52" y="126" width="42" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <rect x="158" y="28" width="84" height="212" rx="12" fill="var(--art-hi)" stroke="var(--art-mid)" stroke-width="1"/>
  <circle cx="200" cy="64" r="13" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  <rect x="194" y="59" width="12" height="10" rx="2.5" stroke="var(--art-edge)" stroke-width="1.4"/>
  <rect x="176" y="96" width="48" height="5.5" rx="2.75" fill="var(--art-deep)"/>
  <rect x="176" y="112" width="38" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <rect x="282" y="28" width="84" height="212" rx="12" fill="var(--art-hi)" stroke="var(--art-mid)" stroke-width="1"/>
  <circle cx="324" cy="64" r="13" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  <path d="m321 59 7 5-7 5Z" fill="var(--art-edge)"/>
  <rect x="300" y="96" width="48" height="5.5" rx="2.75" fill="var(--art-deep)"/>
  <rect x="300" y="112" width="34" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <g class="kense-access-moving">
    <rect x="128" y="158" width="214" height="64" rx="12" fill="var(--art-hi)" stroke="var(--art-edge)" stroke-width="1.2"/>
    <circle cx="154" cy="190" r="5" fill="var(--art-edge)"/>
    <rect x="170" y="181" width="96" height="6" rx="3" fill="var(--art-edge)"/>
    <rect x="170" y="194" width="142" height="4.5" rx="2.25" fill="var(--art-deep)"/>
  </g>
</svg></div></div>"#
        }
        _ => {
            r#"<div class="kense-access-art" data-kind="os" aria-hidden="true"><div class="kense-access-viewport">
<svg viewBox="0 0 400 268" width="340" height="228" preserveAspectRatio="xMaxYMid meet" overflow="visible" fill="none">
  <rect x="44" y="24" width="312" height="216" rx="14" fill="var(--art-hi)" stroke="var(--art-mid)" stroke-width="1"/>
  <line x1="44" y1="62" x2="356" y2="62" stroke="var(--art-mid)" stroke-width="1"/>
  <rect x="64" y="39" width="64" height="6" rx="3" fill="var(--art-deep)"/>
  <rect x="292" y="36" width="44" height="14" rx="7" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  <rect x="64" y="82" width="128" height="62" rx="9" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  <rect x="80" y="98" width="68" height="5.5" rx="2.75" fill="var(--art-deep)"/>
  <rect x="80" y="113" width="92" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <rect x="206" y="82" width="130" height="62" rx="9" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  <rect x="222" y="98" width="72" height="5.5" rx="2.75" fill="var(--art-deep)"/>
  <rect x="222" y="113" width="94" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <rect x="64" y="158" width="272" height="58" rx="9" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  <rect x="80" y="176" width="118" height="5.5" rx="2.75" fill="var(--art-deep)"/>
  <rect x="80" y="191" width="184" height="4.5" rx="2.25" fill="var(--art-mid)"/>
  <g class="kense-access-moving">
    <rect x="192" y="118" width="182" height="92" rx="12" fill="var(--art-hi)" stroke="var(--art-edge)" stroke-width="1.2"/>
    <circle cx="220" cy="152" r="14" fill="var(--art-soft)" stroke="var(--art-edge)" stroke-width="1.2"/>
    <circle cx="220" cy="152" r="4.5" fill="var(--art-edge)"/>
    <rect x="246" y="143" width="94" height="6" rx="3" fill="var(--art-edge)"/>
    <rect x="246" y="156" width="72" height="4.5" rx="2.25" fill="var(--art-deep)"/>
    <rect x="210" y="182" width="144" height="14" rx="7" fill="var(--art-soft)" stroke="var(--art-mid)" stroke-width="1"/>
  </g>
</svg></div></div>"#
        }
    }
}
