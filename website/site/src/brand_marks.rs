//! Bespoke multi-facet folded-ribbon brand logomarks for the Kense product suite.
//!
//! Uses overlapping tonal facets rather than `<defs><linearGradient id=…>` so marks never suffer
//! duplicate DOM ID collisions when rendered inside hidden mega-menus, popovers, and page sections.

/// Monochrome two-tone architectural silhouette for resting navigation states (`viewBox="0 0 64 64"`).
pub fn silhouette_paths(kind: &str) -> &'static str {
    match kind {
        "kui" => {
            r##"<path fill="currentColor" fill-opacity="0.44" fill-rule="evenodd" d="M18 8h16a10 10 0 0 1 10 10v16a10 10 0 0 1-10 10H18A10 10 0 0 1 8 34V18A10 10 0 0 1 18 8Zm1 9a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V19a2 2 0 0 0-2-2H19Z"/><path fill="currentColor" fill-opacity="0.88" fill-rule="evenodd" d="M30 20h16a10 10 0 0 1 10 10v16a10 10 0 0 1-10 10H30a10 10 0 0 1-10-10V30a10 10 0 0 1 10-10Zm1 9a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V31a2 2 0 0 0-2-2H31Z"/>"##
        }
        "kbox" => {
            r##"<path fill="currentColor" fill-opacity="0.42" d="M32 6 54 18.8 44.2 24.5 32 17.4 19.8 24.5 10 18.8Z"/><path fill="currentColor" fill-opacity="0.68" d="M10 18.8 19.8 24.5v15L32 46.6V58L10 45.2Z"/><path fill="currentColor" fill-opacity="0.92" d="M54 18.8v26.4L32 58V46.6L44.2 39.5v-15Z"/><path fill="currentColor" d="m32 23.5 7.6 4.4v8.8L32 41.1l-7.6-4.4v-8.8Z"/>"##
        }
        "ksandbox" => {
            r##"<path fill="currentColor" fill-opacity="0.52" d="M32 6C22 9.5 12 12 10 14.2V30c0 13.5 9.5 23.2 22 28V39.6c-5.2-1.3-8.6-5.7-8.6-11.1 0-5.8 3.7-10.1 8.6-11.1Z"/><path fill="currentColor" fill-opacity="0.9" d="M32 6c10 3.5 20 6 22 8.2V30c0 13.5-9.5 23.2-22 28V39.6c5.2-1.3 8.6-5.7 8.6-11.1 0-5.8-3.7-10.1-8.6-11.1Z"/><path fill="currentColor" d="m32 23 4.8 5.5L32 34l-4.8-5.5Z"/>"##
        }
        "kwork" => {
            r##"<path fill="currentColor" fill-opacity="0.52" fill-rule="evenodd" d="M16 9h32a6 6 0 0 1 6 6v22H10V15a6 6 0 0 1 6-6Zm4 9h11v4.5H20V18Zm0 8h20v4.5H20V26Z"/><path fill="currentColor" fill-opacity="0.9" d="M10 40h44l5 11.5A3.5 3.5 0 0 1 55.8 56H8.2A3.5 3.5 0 0 1 5 51.5Z"/>"##
        }
        "krun" => {
            r##"<path fill="currentColor" fill-opacity="0.38" d="M9 14h14l16 18-16 18H9l16-18Z"/><path fill="currentColor" fill-opacity="0.68" d="M23 14h14l16 18-16 18H23l16-18Z"/><path fill="currentColor" fill-opacity="0.95" d="m38 20 18 12-18 12 6.5-12Z"/>"##
        }
        "aios" => {
            r##"<path fill="currentColor" fill-opacity="0.44" d="M32 6 52 17.5 42.5 23 32 17 21.5 23 12 17.5Z"/><path fill="currentColor" fill-opacity="0.7" d="M54 21v22L35 54v-11l9.5-5.5V26.5Z"/><path fill="currentColor" fill-opacity="0.92" d="M10 21 19.5 26.5v11L29 43v11L10 43Z"/><circle cx="32" cy="32" r="6.2" fill="currentColor"/>"##
        }
        "acp" | "dsh-acp" => {
            r##"<path fill="currentColor" fill-opacity="0.52" d="M10 23c5.5-9.5 16-14 28-11l5-5 9 13-15 3 4.5-4.5c-8.2-1.8-15.4 1.2-19.8 8.5Z"/><path fill="currentColor" fill-opacity="0.9" d="M54 41c-5.5 9.5-16 14-28 11l-5 5-9-13 15-3-4.5 4.5c8.2 1.8 15.4-1.2 19.8-8.5Z"/><path fill="currentColor" d="m32 26.2 5.8 5.8-5.8 5.8-5.8-5.8Z"/>"##
        }
        "kcl" => {
            r##"<path fill="currentColor" fill-opacity="0.55" d="M25 9 8 32l17 23 6.8-5.4L19.4 32l12.4-17.6Z"/><path fill="currentColor" fill-opacity="0.9" d="m39 9 17 23-17 23-6.8-5.4L44.6 32 32.2 14.4Z"/><path fill="currentColor" d="m32 24.5 6.2 7.5-6.2 7.5-6.2-7.5Z"/>"##
        }
        "docs" => {
            r##"<path fill="currentColor" fill-opacity="0.4" d="M18 9h28a6 6 0 0 1 6 6v34a6 6 0 0 1-6 6H18a6 6 0 0 1-6-6V15a6 6 0 0 1 6-6Z"/><path fill="currentColor" fill-opacity="0.88" d="M12 15h28a6 6 0 0 1 6 6v30a6 6 0 0 1-6 6H12a6 6 0 0 1-6-6V21a6 6 0 0 1 6-6Z"/>"##
        }
        _ => {
            r##"<rect x="8" y="10" width="48" height="44" rx="11" fill="currentColor" fill-opacity="0.86"/><path d="m21 24 8 8-8 8m12 0h10" stroke="var(--bg, #fff)" stroke-width="4.5" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##
        }
    }
}

/// Full multi-facet jewel-tone sculpted ribbon paths (`viewBox="0 0 64 64"`).
pub fn detail_paths(kind: &str) -> &'static str {
    match kind {
        "kui" => {
            // Interlocking Sapphire & Sky-Cobalt Escher Ribbon Frames + Precision Core Spark
            r##"<path fill="#60A5FA" fill-rule="evenodd" d="M18 8h16a10 10 0 0 1 10 10v16a10 10 0 0 1-10 10H18A10 10 0 0 1 8 34V18A10 10 0 0 1 18 8Zm1 9a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V19a2 2 0 0 0-2-2H19Z"/><path fill="#2563EB" fill-rule="evenodd" d="M30 20h16a10 10 0 0 1 10 10v16a10 10 0 0 1-10 10H30a10 10 0 0 1-10-10V30a10 10 0 0 1 10-10Zm1 9a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V31a2 2 0 0 0-2-2H31Z"/><path fill="#1E3A8A" d="M35 20h9v9h-9z" opacity="0.32"/><path fill="#93C5FD" d="M35 16h9v15h-9z"/><path fill="#1D4ED8" d="M20 35h9v9h-9z"/><path fill="#DBEAFE" d="m32 27.2 4.8 4.8-4.8 4.8-4.8-4.8Z"/>"##
        }
        "kbox" => {
            // 3-Wing Origami Emerald Hex-Vault + Floating 3-Facet MicroVM Die
            r##"<path fill="#6EE7B7" d="M32 6 54 18.8 44.2 24.5 32 17.4 19.8 24.5 10 18.8Z"/><path fill="#10B981" d="M10 18.8 19.8 24.5v15L32 46.6V58L10 45.2Z"/><path fill="#047857" d="M54 18.8v26.4L32 58V46.6L44.2 39.5v-15Z"/><path fill="#A7F3D0" d="m32 22.8 8.2 4.7-8.2 4.7-8.2-4.7Z"/><path fill="#34D399" d="m23.8 27.5 8.2 4.7v9.4l-8.2-4.7Z"/><path fill="#059669" d="m40.2 27.5-8.2 4.7v9.4l8.2-4.7Z"/>"##
        }
        "ksandbox" => {
            // Aerodynamic Split-Blade Cobalt Aegis + Luminous Crown & Lock Core
            r##"<path fill="#38BDF8" d="M32 6C22 9.5 12 12 10 14.2V30c0 13.5 9.5 23.2 22 28V39.6c-5.2-1.3-8.6-5.7-8.6-11.1 0-5.8 3.7-10.1 8.6-11.1Z"/><path fill="#0284C7" d="M32 6c10 3.5 20 6 22 8.2V30c0 13.5-9.5 23.2-22 28V39.6c5.2-1.3 8.6-5.7 8.6-11.1 0-5.8-3.7-10.1-8.6-11.1Z"/><path fill="#BAE6FD" d="M32 6 47 11.4 32 17.4 17 11.4Z"/><path fill="#E0F2FE" d="m32 22.8 5.2 5.7-5.2 5.7-5.2-5.7Z"/><path fill="#0369A1" d="m32 22.8 5.2 5.7-5.2 5.7Z" opacity="0.45"/>"##
        }
        "kwork" => {
            // Cantilevered Burnished-Amber Studio Display + Keyboard Fold
            r##"<path fill="#FBBF24" d="M16 9h32a6 6 0 0 1 6 6v23H10V15a6 6 0 0 1 6-6Z"/><path fill="#FDE68A" d="M16 9h32a6 6 0 0 1 6 6v5H10v-5a6 6 0 0 1 6-6Z"/><path fill="#B45309" d="M10 36h44v4H10Z"/><path fill="#D97706" d="M10 39h44l5.2 11.8A3.4 3.4 0 0 1 56 55.6H8a3.4 3.4 0 0 1-3.2-4.8Z"/><path fill="#78350F" d="m19 20 6.5 5-6.5 5v-3.4l3.2-1.6-3.2-1.6Zm9.5 6.8h15v3.4h-15Z"/><rect x="24" y="45.5" width="16" height="4" rx="2" fill="#FEF3C7"/>"##
        }
        "krun" => {
            // Supersonic Triple-Delta Coral/Vermilion Launch Wing
            r##"<path fill="#FDA4AF" d="M8 14h14l16 18-16 18H8l16-18Z"/><path fill="#FB7185" d="M21 14h14.5l16 18-16 18H21l16-18Z"/><path fill="#E11D48" d="m35.5 18 20.5 14-20.5 14 7.5-14Z"/><path fill="#9F1239" d="m43 32 13 0-20.5 14Z"/>"##
        }
        "aios" => {
            // 120-Degree Symmetrical Amethyst Control Turbine + Singular Key Core
            r##"<path fill="#C4B5FD" d="M32 6 52.8 18 43 23.6 32 17.2 21 23.6 11.2 18Z"/><path fill="#8B5CF6" d="M54.5 21.2V45.2L33.8 57.2V45.8L44.7 39.5V26.8Z"/><path fill="#5B21B6" d="M9.5 21.2 19.3 26.8V39.5L30.2 45.8V57.2L9.5 45.2Z"/><path fill="#EDE9FE" d="m32 23.2 7.6 4.4v8.8L32 40.8l-7.6-4.4v-8.8Z"/><circle cx="32" cy="32" r="3.6" fill="#4C1D95"/>"##
        }
        "acp" | "dsh-acp" => {
            // Interlocking Teal/Sea-Glass Duplex Protocol Ribbons
            r##"<path fill="#5EEAD4" d="M10 24C15.5 13.5 26.5 9 39 12.2l4.6-4.7L53 21l-15.5 3.2 4.6-4.6c-8.2-1.8-15.6 1.2-20.1 8.6Z"/><path fill="#0D9488" d="M54 40c-5.5 10.5-16.5 15-29 11.8L20.4 56.5 11 43l15.5-3.2-4.6 4.6c8.2 1.8 15.6-1.2 20.1-8.6Z"/><path fill="#14B8A6" d="m32 24.8 7.2 7.2-7.2 7.2-7.2-7.2Z"/><path fill="#CCFBF1" d="m32 28.2 3.8 3.8-3.8 3.8-3.8-3.8Z"/>"##
        }
        "kcl" => {
            // Precision Chamfered Platinum/Indigo Brackets + Teal Validation Crystal
            r##"<path fill="#94A3B8" d="M25 9 8 32h11.4L31.8 14.4Z"/><path fill="#475569" d="M8 32l17 23 6.8-5.4L19.4 32Z"/><path fill="#CBD5E1" d="m39 9 17 23H44.6L32.2 14.4Z"/><path fill="#64748B" d="M56 32 39 55l-6.8-5.4L44.6 32Z"/><path fill="#2DD4BF" d="m32 23.5 6.8 8.5-6.8 8.5-6.8-8.5Z"/><path fill="#0F766E" d="m32 23.5 6.8 8.5-6.8 8.5Z" opacity="0.42"/>"##
        }
        "docs" => {
            // Layered Cobalt Architectural Folios
            r##"<rect x="17" y="8" width="34" height="42" rx="7" fill="#93C5FD"/><rect x="11" y="14" width="35" height="42" rx="7" fill="#2563EB"/><rect x="18" y="24" width="19" height="3.6" rx="1.8" fill="#EFF6FF"/><rect x="18" y="31.5" width="14" height="3.6" rx="1.8" fill="#BFDBFE"/><rect x="18" y="39" width="16.5" height="3.6" rx="1.8" fill="#93C5FD"/>"##
        }
        _ => {
            // Sculpted Graphite CLI Tile + Emerald Prompt & Amber Cursor
            r##"<rect x="8" y="10" width="48" height="44" rx="11" fill="#1E293B"/><path fill="#334155" d="M19 10h26a11 11 0 0 1 11 11v4H8v-4A11 11 0 0 1 19 10Z"/><path d="m20 26 8.5 7-8.5 7" stroke="#34D399" stroke-width="4.6" stroke-linecap="round" stroke-linejoin="round" fill="none"/><rect x="33" y="37" width="11" height="4.4" rx="2.2" fill="#FBBF24"/>"##
        }
    }
}

/// Standalone full-color jewel brand mark (`<svg class="kense-emblem-svg" ...>`).
pub fn emblem(kind: &str, size: u8) -> String {
    format!(
        r#"<svg class="kense-emblem-svg" width="{size}" height="{size}" viewBox="0 0 64 64" fill="none" aria-hidden="true">{}</svg>"#,
        detail_paths(kind)
    )
}

/// Two-layer morphing topbar mega-menu mark (resting silhouette -> full jewel detail on hover).
pub fn mega_logo(kind: &str) -> String {
    format!(
        r#"<span class="kense-mega-logo" data-mega-logo aria-hidden="true"><svg class="logo-silhouette" viewBox="0 0 64 64" fill="none">{}</svg><svg class="logo-detail" viewBox="0 0 64 64" fill="none">{}</svg></span>"#,
        silhouette_paths(kind),
        detail_paths(kind),
    )
}
