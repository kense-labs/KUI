//! App directory with sticky filter bar, brand marks, connect states, and detail inspector.
//!
//! Renders an [`AppShell`] integrations marketplace with status filter tabs (`All`,
//! `Connected`, `Available`), live search, `+ Request` popover form, two-column
//! categorized brand rows (`Popular`, `Google`, `Communication`), expandable overflow
//! groups, loading skeletons, empty states, and a right-hand permissions inspector.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, ProTone, pro_app_tile, pro_badge, pro_btn, pro_eid,
            pro_empty_state, pro_fold, pro_mark_selected, pro_pill_indicator, pro_presence,
            pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_switch, pro_toast_banner,
            pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

// Verified rosette badge & brand mark inner SVG paths from reference spec.
const ICON_VERIFIED_BADGE: &str = r#"<path d="M18.9905 19H19M18.9905 19C18.3678 19.6175 17.2393 19.4637 16.4479 19.4637C15.4765 19.4637 15.0087 19.6537 14.3154 20.347C13.7251 20.9374 12.9337 22 12 22C11.0663 22 10.2749 20.9374 9.68457 20.347C8.99128 19.6537 8.52349 19.4637 7.55206 19.4637C6.76068 19.4637 5.63218 19.6175 5.00949 19C4.38181 18.3776 4.53628 17.2444 4.53628 16.4479C4.53628 15.4414 4.31616 14.9786 3.59938 14.2618C2.53314 13.1956 2.00002 12.6624 2 12C2.00001 11.3375 2.53312 10.8044 3.59935 9.73817C4.2392 9.09832 4.53628 8.46428 4.53628 7.55206C4.53628 6.76065 4.38249 5.63214 5 5.00944C5.62243 4.38178 6.7556 4.53626 7.55208 4.53626C8.46427 4.53626 9.09832 4.2392 9.73815 3.59937C10.8044 2.53312 11.3375 2 12 2C12.6625 2 13.1956 2.53312 14.2618 3.59937C14.9015 4.23907 15.5355 4.53626 16.4479 4.53626C17.2393 4.53626 18.3679 4.38247 18.9906 5C19.6182 5.62243 19.4637 6.75559 19.4637 7.55206C19.4637 8.55858 19.6839 9.02137 20.4006 9.73817C21.4669 10.8044 22 11.3375 22 12C22 12.6624 21.4669 13.1956 20.4006 14.2618C19.6838 14.9786 19.4637 15.4414 19.4637 16.4479C19.4637 17.2444 19.6182 18.3776 18.9905 19Z" stroke="currentColor" stroke-width="1.9"></path><path d="M9 12.8929C9 12.8929 10.2 13.5447 10.8 14.5C10.8 14.5 12.6 10.75 15 9.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const MARK_NOTION: &str = r#"<path fill="currentColor" d="M4.459 4.208c.746.606 1.026.56 2.428.466l13.215-.793c.28 0 .047-.28-.046-.326L17.86 1.968c-.42-.326-.981-.7-2.055-.607L3.01 2.295c-.466.046-.56.28-.374.466zm.793 3.08v13.904c0 .747.373 1.027 1.214.98l14.523-.84c.841-.046.935-.56.935-1.167V6.354c0-.606-.233-.933-.748-.887l-15.177.887c-.56.047-.747.327-.747.933zm14.337.745c.093.42 0 .84-.42.888l-.7.14v10.264c-.608.327-1.168.514-1.635.514-.748 0-.935-.234-1.495-.933l-4.577-7.186v6.952L12.21 19s0 .84-1.168.84l-3.222.186c-.093-.186 0-.653.327-.746l.84-.233V9.854L7.822 9.76c-.094-.42.14-1.026.793-1.073l3.456-.233 4.764 7.279v-6.44l-1.215-.139c-.093-.514.28-.887.747-.933zM1.936 1.035l13.31-.98c1.634-.14 2.055-.047 3.082.7l4.249 2.986c.7.513.934.653.934 1.213v16.378c0 1.026-.373 1.634-1.68 1.726l-15.458.934c-.98.047-1.448-.093-1.962-.747l-3.129-4.06c-.56-.747-.793-1.306-.793-1.96V2.667c0-.839.374-1.54 1.447-1.632z"></path>"#;

const MARK_LINEAR: &str = r#"<path fill="currentColor" d="M2.886 4.18A11.982 11.982 0 0 1 11.99 0C18.624 0 24 5.376 24 12.009c0 3.64-1.62 6.903-4.18 9.105L2.887 4.18ZM1.817 5.626l16.556 16.556c-.524.33-1.075.62-1.65.866L.951 7.277c.247-.575.537-1.126.866-1.65ZM.322 9.163l14.515 14.515c-.71.172-1.443.282-2.195.322L0 11.358a12 12 0 0 1 .322-2.195Zm-.17 4.862 9.823 9.824a12.02 12.02 0 0 1-9.824-9.824Z"></path>"#;

const MARK_FIGMA_COLOR: &str = r##"<path d="M19 28.5a9.5 9.5 0 1 1 19 0 9.5 9.5 0 0 1-19 0Z" fill="#1ABCFE"></path><path d="M0 47.5A9.5 9.5 0 0 1 9.5 38H19v9.5a9.5 9.5 0 1 1-19 0Z" fill="#0ACF83"></path><path d="M19 0v19h9.5a9.5 9.5 0 1 0 0-19H19Z" fill="#FF7262"></path><path d="M0 9.5A9.5 9.5 0 0 0 9.5 19H19V0H9.5A9.5 9.5 0 0 0 0 9.5Z" fill="#F24E1E"></path><path d="M0 28.5A9.5 9.5 0 0 0 9.5 38H19V19H9.5A9.5 9.5 0 0 0 0 28.5Z" fill="#A259FF"></path>"##;

const MARK_GITHUB: &str = r#"<path fill="currentColor" d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"></path>"#;

const MARK_HUBSPOT: &str = r#"<path fill="currentColor" d="M18.164 7.93V5.084a2.198 2.198 0 001.267-1.978v-.067A2.2 2.2 0 0017.238.845h-.067a2.2 2.2 0 00-2.193 2.193v.067a2.196 2.196 0 001.252 1.973l.013.006v2.852a6.22 6.22 0 00-2.969 1.31l.012-.01-7.828-6.095A2.497 2.497 0 104.3 4.656l-.012.006 7.697 5.991a6.176 6.176 0 00-1.038 3.446c0 1.343.425 2.588 1.147 3.607l-.013-.02-2.342 2.343a1.968 1.968 0 00-.58-.095h-.002a2.033 2.033 0 102.033 2.033 1.978 1.978 0 00-.1-.595l.005.014 2.317-2.317a6.247 6.247 0 104.782-11.134l-.036-.005zm-.964 9.378a3.206 3.206 0 113.215-3.207v.002a3.206 3.206 0 01-3.207 3.207z"></path>"#;

const MARK_STRIPE: &str = r#"<path fill="currentColor" d="M13.976 9.15c-2.172-.806-3.356-1.426-3.356-2.409 0-.831.683-1.305 1.901-1.305 2.227 0 4.515.858 6.09 1.631l.89-5.494C18.252.975 15.697 0 12.165 0 9.667 0 7.589.654 6.104 1.872 4.56 3.147 3.757 4.992 3.757 7.218c0 4.039 2.467 5.76 6.476 7.219 2.585.92 3.445 1.574 3.445 2.583 0 .98-.84 1.545-2.354 1.545-1.875 0-4.965-.921-6.99-2.109l-.9 5.555C5.175 22.99 8.385 24 11.714 24c2.641 0 4.843-.624 6.328-1.813 1.664-1.305 2.525-3.236 2.525-5.732 0-4.128-2.524-5.851-6.594-7.305h.003z"></path>"#;

const MARK_ZAPIER: &str = r#"<path fill="currentColor" d="M18.5 10.75H14.56l2.79-2.79-1.77-1.77-2.79 2.79V4.99h-2.5v3.99L7.5 6.19 5.73 7.96l2.79 2.79H4.53v2.5h3.99l-2.79 2.79 1.77 1.77 2.79-2.79v3.99h2.5v-3.99l2.79 2.79 1.77-1.77-2.79-2.79h3.94v-2.5Z"></path>"#;

const MARK_AIRTABLE: &str = r#"<path fill="currentColor" d="M11.992 1.966c-.434 0-.87.086-1.28.257L1.779 5.917c-.503.208-.49.908.012 1.116l8.982 3.558a3.266 3.266 0 0 0 2.454 0l8.982-3.558c.503-.196.503-.908.012-1.116l-8.957-3.694a3.255 3.255 0 0 0-1.272-.257zM23.4 8.056a.589.589 0 0 0-.222.045l-10.012 3.877a.612.612 0 0 0-.38.564v8.896a.6.6 0 0 0 .821.552L23.62 18.1a.583.583 0 0 0 .38-.551V8.653a.6.6 0 0 0-.6-.596zM.676 8.095a.644.644 0 0 0-.48.19C.086 8.396 0 8.53 0 8.69v8.355c0 .442.515.737.908.54l6.27-3.006.307-.147 2.969-1.436c.466-.22.43-.908-.061-1.092L.883 8.138a.57.57 0 0 0-.207-.044z"></path>"#;

const MARK_GOOGLE_ANALYTICS: &str = r#"<path fill="currentColor" d="M22.84 2.9982v17.9987c.0086 1.6473-1.3197 2.9897-2.967 2.9984a2.9808 2.9808 0 01-.3677-.0208c-1.528-.226-2.6477-1.5558-2.6105-3.1V3.1204c-.0369-1.5458 1.0856-2.8762 2.6157-3.1 1.6361-.1915 3.1178.9796 3.3093 2.6158.014.1201.0208.241.0202.3619zM4.1326 18.0548c-1.6417 0-2.9726 1.331-2.9726 2.9726C1.16 22.6691 2.4909 24 4.1326 24s2.9726-1.3309 2.9726-2.9726-1.331-2.9726-2.9726-2.9726zm7.8728-9.0098c-.0171 0-.0342 0-.0513.0003-1.6495.0904-2.9293 1.474-2.891 3.1256v7.9846c0 2.167.9535 3.4825 2.3505 3.763 1.6118.3266 3.1832-.7152 3.5098-2.327.04-.1974.06-.3983.0593-.5998v-8.9585c.003-1.6474-1.33-2.9852-2.9773-2.9882z"></path>"#;

const MARK_ZOOM: &str = r#"<path fill="currentColor" d="M5.033 14.649H.743a.74.74 0 0 1-.686-.458.74.74 0 0 1 .16-.808L3.19 10.41H1.06A1.06 1.06 0 0 1 0 9.35h3.957c.301 0 .57.18.686.458a.74.74 0 0 1-.161.808L1.51 13.59h2.464c.585 0 1.06.475 1.06 1.06zM24 11.338c0-1.14-.927-2.066-2.066-2.066-.61 0-1.158.265-1.537.686a2.061 2.061 0 0 0-1.536-.686c-1.14 0-2.066.926-2.066 2.066v3.311a1.06 1.06 0 0 0 1.06-1.06v-2.251a1.004 1.004 0 0 1 2.013 0v2.251c0 .586.474 1.06 1.06 1.06v-3.311a1.004 1.004 0 0 1 2.012 0v2.251c0 .586.475 1.06 1.06 1.06zM16.265 12a2.728 2.728 0 1 1-5.457 0 2.728 2.728 0 0 1 5.457 0zm-1.06 0a1.669 1.669 0 1 0-3.338 0 1.669 1.669 0 0 0 3.338 0zm-4.82 0a2.728 2.728 0 1 1-5.458 0 2.728 2.728 0 0 1 5.457 0zm-1.06 0a1.669 1.669 0 1 0-3.338 0 1.669 1.669 0 0 0 3.338 0z"></path>"#;

const MARK_DISCORD: &str = r#"<path fill="currentColor" d="M20.317 4.3698a19.7913 19.7913 0 00-4.8851-1.5152.0741.0741 0 00-.0785.0371c-.211.3753-.4447.8648-.6083 1.2495-1.8447-.2762-3.68-.2762-5.4868 0-.1636-.3933-.4058-.8742-.6177-1.2495a.077.077 0 00-.0785-.037 19.7363 19.7363 0 00-4.8852 1.515.0699.0699 0 00-.0321.0277C.5334 9.0458-.319 13.5799.0992 18.0578a.0824.0824 0 00.0312.0561c2.0528 1.5076 4.0413 2.4228 5.9929 3.0294a.0777.0777 0 00.0842-.0276c.4616-.6304.8731-1.2952 1.226-1.9942a.076.076 0 00-.0416-.1057c-.6528-.2476-1.2743-.5495-1.8722-.8923a.077.077 0 01-.0076-.1277c.1258-.0943.2517-.1923.3718-.2914a.0743.0743 0 01.0776-.0105c3.9278 1.7933 8.18 1.7933 12.0614 0a.0739.0739 0 01.0785.0095c.1202.099.246.1981.3728.2924a.077.077 0 01-.0066.1276 12.2986 12.2986 0 01-1.873.8914.0766.0766 0 00-.0407.1067c.3604.698.7719 1.3628 1.225 1.9932a.076.076 0 00.0842.0286c1.961-.6067 3.9495-1.5219 6.0023-3.0294a.077.077 0 00.0313-.0552c.5004-5.177-.8382-9.6739-3.5485-13.6604a.061.061 0 00-.0312-.0286zM8.02 15.3312c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9555-2.4189 2.157-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.9555 2.4189-2.1569 2.4189zm7.9748 0c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9554-2.4189 2.1569-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.946 2.4189-2.1568 2.4189Z"></path>"#;

const MARK_INTERCOM: &str = r#"<path fill="currentColor" d="M21 0H3C1.343 0 0 1.343 0 3v18c0 1.658 1.343 3 3 3h18c1.658 0 3-1.342 3-3V3c0-1.657-1.342-3-3-3zm-5.801 4.399c0-.44.36-.8.802-.8.44 0 .8.36.8.8v10.688c0 .442-.36.801-.8.801-.443 0-.802-.359-.802-.801V4.399zM11.2 3.994c0-.44.357-.799.8-.799s.8.359.8.799v11.602c0 .44-.357.8-.8.8s-.8-.36-.8-.8V3.994zm-4 .405c0-.44.359-.8.799-.8.443 0 .802.36.802.8v10.688c0 .442-.36.801-.802.801-.44 0-.799-.359-.799-.801V4.399zM3.199 6c0-.442.36-.8.802-.8.44 0 .799.358.799.8v7.195c0 .441-.359.8-.799.8-.443 0-.802-.36-.802-.8V6zM20.52 18.202c-.123.105-3.086 2.593-8.52 2.593-5.433 0-8.397-2.486-8.521-2.593-.335-.288-.375-.792-.086-1.128.285-.334.79-.375 1.125-.09.047.041 2.693 2.211 7.481 2.211 4.848 0 7.456-2.186 7.479-2.207.334-.289.839-.25 1.128.086.289.336.25.84-.086 1.128zm.281-5.007c0 .441-.36.8-.801.8-.441 0-.801-.36-.801-.8V6c0-.442.361-.8.801-.8.441 0 .801.357.801.8v7.195z"></path>"#;

const MARK_CALENDLY: &str = r#"<path fill="currentColor" d="M19.655 14.262c.281 0 .557.023.828.064 0 .005-.005.01-.005.014-.105.267-.234.534-.381.786l-1.219 2.106c-1.112 1.936-3.177 3.127-5.411 3.127h-2.432c-2.23 0-4.294-1.191-5.412-3.127l-1.218-2.106a6.251 6.251 0 0 1 0-6.252l1.218-2.106C6.736 4.832 8.8 3.641 11.035 3.641h2.432c2.23 0 4.294 1.191 5.411 3.127l1.219 2.106c.147.252.271.519.381.786 0 .004.005.009.005.014-.267.041-.543.064-.828.064-1.816 0-2.501-.607-3.291-1.306-.764-.676-1.711-1.517-3.44-1.517h-1.029c-1.251 0-2.387.455-3.2 1.278-.796.805-1.233 1.904-1.233 3.099v1.411c0 1.196.437 2.295 1.233 3.099.813.823 1.949 1.278 3.2 1.278h1.034c1.729 0 2.676-.841 3.439-1.517.791-.703 1.471-1.306 3.287-1.301Z"></path>"#;

const MARK_TYPEFORM: &str = r#"<path fill="currentColor" d="M3.2 7.5H0v1.8h2.2v7.2h2V9.3h2.2V7.5H3.2zm14.6 1.9c-2.3 0-3.9 1.6-3.9 3.8s1.6 3.8 3.9 3.8 3.9-1.6 3.9-3.8-1.6-3.8-3.9-3.8zm0 5.9c-1.2 0-2-.9-2-2.1s.8-2.1 2-2.1 2 .9 2 2.1-.8 2.1-2 2.1zm-8.3-5.9c-2.2 0-3.7 1.6-3.7 3.8s1.5 3.8 3.8 3.8c1.5 0 2.7-.7 3.3-1.8l-1.6-.9c-.3.6-.9 1-1.7 1-1 0-1.8-.6-1.9-1.6h5.3v-.6c0-2.2-1.4-3.7-3.5-3.7zm-1.8 3c.2-.9.8-1.4 1.8-1.4.9 0 1.6.5 1.7 1.4H7.7z"></path>"#;

const MARK_DROPBOX: &str = r#"<path fill="currentColor" d="M6 1.807L0 5.629l6 3.822 6.001-3.822L6 1.807zM18 1.807l-6 3.822 6 3.822 6-3.822-6-3.822zM0 13.274l6 3.822 6.001-3.822L6 9.452l-6 3.822zM18 9.452l-6 3.822 6 3.822 6-3.822-6-3.822zM6 18.371l6.001 3.822 6-3.822-6-3.822L6 18.371z"></path>"#;

const MARK_MIXPANEL: &str = r#"<path fill="currentColor" d="M6.967 9.996h3.053c-.763-.477-1.048-1.145-1.431-2.384L7.443 3.366C6.919 1.458 6.49.551 4.39.551H.004v1.145h.621c1.286 0 1.431.477 1.814 1.908L3.44 7.326c.524 1.814 1.337 2.67 3.53 2.67h-.003Zm7.06 0h3.053c2.194 0 2.956-.86 3.484-2.67l1.001-3.722c.382-1.431.57-1.908 1.814-1.908H24V.551h-4.34c-2.146 0-2.576.86-3.053 2.815l-1.145 4.246c-.384 1.286-.673 1.907-1.435 2.384Zm-4.007 4.008h4.007V9.996H10.02v4.008ZM0 23.449h4.39c2.1 0 2.529-.907 3.053-2.815l1.146-4.246c.383-1.239.668-1.907 1.431-2.384H6.967c-2.194 0-3.007.86-3.531 2.67l-1.001 3.722c-.383 1.431-.524 1.907-1.814 1.907H0v1.146Zm19.65 0h4.343v-1.146h-.622c-1.239 0-1.431-.476-1.814-1.907l-1.001-3.722c-.524-1.814-1.286-2.67-3.483-2.67h-3.046c.762.477 1.041 1.098 1.424 2.384l1.145 4.246c.477 1.955.907 2.815 3.054 2.815Z"></path>"#;

const MARK_MAKE: &str = r#"<path fill="currentColor" d="M13.38 3.498c-.27 0-.511.19-.566.465L9.85 18.986a.578.578 0 0 0 .453.678l4.095.826a.58.58 0 0 0 .682-.455l2.963-15.021a.578.578 0 0 0-.453-.678l-4.096-.826a.589.589 0 0 0-.113-.012zm-5.876.098a.576.576 0 0 0-.516.318L.062 17.697a.575.575 0 0 0 .256.774l3.733 1.877a.578.578 0 0 0 .775-.258l6.926-13.781a.577.577 0 0 0-.256-.776L7.762 3.658a.571.571 0 0 0-.258-.062zm11.74.115a.576.576 0 0 0-.576.576v15.426c0 .318.258.578.576.578h4.178a.58.58 0 0 0 .578-.578V4.287a.578.578 0 0 0-.578-.576Z"></path>"#;

const MARK_MIRO: &str = r#"<path fill="currentColor" d="M17.392 3H14.55l2.63 4.8L11.79 3H8.95l2.68 5.6L6.2 3H3.35l2.89 7.2L3.35 21h2.85l5.43-10.2L8.95 21h2.84l5.39-11.2L14.55 21h2.84L21 9.4 17.392 3Z"></path>"#;

/// Connection filter tab on [`IntegrationsScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IntegrationFilter {
    #[default]
    All,
    Connected,
    Available,
}

impl IntegrationFilter {
    /// Ordered tabs rendered in the sticky filter bar.
    pub const ALL: [Self; 3] = [Self::All, Self::Connected, Self::Available];

    /// Localized tab label.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All", "全部"),
            Self::Connected => tr(cx, "Connected", "已连接"),
            Self::Available => tr(cx, "Available", "可连接"),
        }
    }

    fn as_id(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Connected => "connected",
            Self::Available => "available",
        }
    }
}

/// Directory section grouping [`IntegrationItem`]s.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IntegrationCategory {
    #[default]
    Popular,
    Google,
    Communication,
}

impl IntegrationCategory {
    /// Ordered sections shown on [`IntegrationsScreen`].
    pub const ALL: [Self; 3] = [Self::Popular, Self::Google, Self::Communication];

    /// Localized section heading.
    pub fn title(self, cx: &App) -> &'static str {
        match self {
            Self::Popular => tr(cx, "Popular", "常用推荐"),
            Self::Google => "Google",
            Self::Communication => tr(cx, "Communication", "沟通与表单"),
        }
    }

    /// Stable slug identifier.
    pub fn as_id(self) -> &'static str {
        match self {
            Self::Popular => "popular",
            Self::Google => "google",
            Self::Communication => "communication",
        }
    }
}

/// One OAuth permission scope exposed in the integration detail inspector.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IntegrationScope {
    pub key: SharedString,
    pub description: SharedString,
    pub enabled: bool,
}

impl IntegrationScope {
    /// Construct an enabled OAuth permission scope.
    pub fn new(key: impl Into<SharedString>, description: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
            enabled: true,
        }
    }
}

/// One app card in [`IntegrationsScreen`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IntegrationItem {
    pub id: SharedString,
    pub name: SharedString,
    pub description: SharedString,
    pub category: IntegrationCategory,
    pub verified: bool,
    pub connected: bool,
    pub collapsed_by_default: bool,
    pub account: SharedString,
    pub sync_status: SharedString,
    pub scopes: Vec<IntegrationScope>,
}

impl IntegrationItem {
    /// Create an integration item with default read/write workspace scopes.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        description: impl Into<SharedString>,
        category: IntegrationCategory,
    ) -> Self {
        let name_s: SharedString = name.into();
        Self {
            id: id.into(),
            name: name_s.clone(),
            description: description.into(),
            category,
            verified: false,
            connected: false,
            collapsed_by_default: false,
            account: "alex@acme.ai".into(),
            sync_status: "Synced 2m ago · Webhook 99.98%".into(),
            scopes: vec![
                IntegrationScope::new(
                    "records:read",
                    format!("Read linked {name_s} items and metadata in chat"),
                ),
                IntegrationScope::new(
                    "records:write",
                    format!("Create and update {name_s} actions from threads"),
                ),
                IntegrationScope::new(
                    "webhooks:receive",
                    format!("Deliver live {name_s} activity events"),
                ),
            ],
        }
    }

    /// Mark the integration with a verified publisher rosette.
    pub fn with_verified(mut self, verified: bool) -> Self {
        self.verified = verified;
        self
    }

    /// Set the initial connection state.
    pub fn with_connected(mut self, connected: bool) -> Self {
        self.connected = connected;
        self
    }

    /// Collapse this item under its section's `"and N more"` disclosure until expanded or searched.
    pub fn with_collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed_by_default = collapsed;
        self
    }

    /// Override OAuth scopes displayed in the right-hand inspector.
    pub fn with_scopes(mut self, scopes: Vec<IntegrationScope>) -> Self {
        self.scopes = scopes;
        self
    }
}

/// Events emitted by [`IntegrationsScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrationsEvent {
    Navigate(SharedString),
    FilterChanged(IntegrationFilter),
    ConnectionToggled {
        id: SharedString,
        connected: bool,
    },
    Selected(SharedString),
    ScopeToggled {
        id: SharedString,
        scope: SharedString,
        enabled: bool,
    },
    IntegrationRequested(SharedString),
    WebhookTested(SharedString),
}

impl EventEmitter<IntegrationsEvent> for IntegrationsScreen {}

fn render_integration_logo(id: &str, name: &str, side: f32, cx: &App) -> AnyElement {
    let p = ProPalette::get(cx);
    let key = id.trim().to_ascii_lowercase();
    let compact_tile = side <= 22.;
    let radius = if compact_tile { 6. } else { 8. };
    let glyph = if compact_tile { 12. } else { 18. };
    let bare_glyph = if compact_tile { 13. } else { 20. };

    let solid_tile = |bg: Hsla, mark: AnyElement, border: Option<Hsla>| -> AnyElement {
        div()
            .size(px(side))
            .flex_none()
            .rounded(px(radius))
            .bg(bg)
            .when_some(border, |d, bc| d.border_1().border_color(bc))
            .flex()
            .items_center()
            .justify_center()
            .child(mark)
            .into_any_element()
    };

    let open_mark = |mark: AnyElement| -> AnyElement {
        if compact_tile {
            solid_tile(p.card, mark, Some(p.line))
        } else {
            div()
                .size(px(side))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(mark)
                .into_any_element()
        }
    };

    match key.as_str() {
        "slack" => {
            if let Some(art) = pro_app_artwork("slack", bare_glyph, cx) {
                open_mark(art)
            } else {
                pro_app_tile("slack", side, cx)
            }
        }
        "notion" => open_mark(pro_svg("0 0 24 24", MARK_NOTION, bare_glyph, p.ink)),
        "linear" => solid_tile(
            rgb(0x5e6ad2).into(),
            pro_svg("0 0 24 24", MARK_LINEAR, glyph, rgb(0xffffff).into()),
            None,
        ),
        "figma" => open_mark(pro_svg(
            "0 0 38 57",
            MARK_FIGMA_COLOR,
            bare_glyph,
            rgb(0xffffff).into(),
        )),
        "github" => solid_tile(
            rgb(0x1f2328).into(),
            pro_svg("0 0 24 24", MARK_GITHUB, glyph, rgb(0xffffff).into()),
            p.dark.then_some(p.line_strong),
        ),
        "hubspot" => open_mark(pro_svg(
            "0 0 24 24",
            MARK_HUBSPOT,
            bare_glyph,
            rgb(0xff7a59).into(),
        )),
        "stripe" => solid_tile(
            rgb(0x635bff).into(),
            pro_svg("0 0 24 24", MARK_STRIPE, glyph, rgb(0xffffff).into()),
            None,
        ),
        "zapier" => solid_tile(
            rgb(0xff4f00).into(),
            pro_svg("0 0 24 24", MARK_ZAPIER, glyph, rgb(0xffffff).into()),
            None,
        ),
        "airtable" => open_mark(pro_svg(
            "0 0 24 24",
            MARK_AIRTABLE,
            bare_glyph,
            rgb(0x18bfff).into(),
        )),
        "google-drive" | "google drive" => {
            if let Some(art) = pro_app_artwork("google drive", bare_glyph, cx) {
                open_mark(art)
            } else {
                pro_app_tile("google drive", side, cx)
            }
        }
        "google-calendar" | "google calendar" => {
            if let Some(art) = pro_app_artwork("google calendar", bare_glyph, cx) {
                open_mark(art)
            } else {
                pro_app_tile("google calendar", side, cx)
            }
        }
        "google-analytics" | "google analytics" => open_mark(pro_svg(
            "0 0 24 24",
            MARK_GOOGLE_ANALYTICS,
            bare_glyph,
            rgb(0xe37400).into(),
        )),
        "zoom" => solid_tile(
            rgb(0x0b5cff).into(),
            pro_svg("0 0 24 24", MARK_ZOOM, glyph, rgb(0xffffff).into()),
            None,
        ),
        "discord" => solid_tile(
            rgb(0x5865f2).into(),
            pro_svg("0 0 24 24", MARK_DISCORD, glyph, rgb(0xffffff).into()),
            None,
        ),
        "intercom" => solid_tile(
            rgb(0x1f8ded).into(),
            pro_svg("0 0 24 24", MARK_INTERCOM, glyph, rgb(0xffffff).into()),
            None,
        ),
        "calendly" => solid_tile(
            rgb(0x006bff).into(),
            pro_svg("0 0 24 24", MARK_CALENDLY, glyph, rgb(0xffffff).into()),
            None,
        ),
        "typeform" => solid_tile(
            rgb(0x262627).into(),
            pro_svg("0 0 24 24", MARK_TYPEFORM, glyph, rgb(0xffffff).into()),
            p.dark.then_some(p.line_strong),
        ),
        "dropbox" => open_mark(pro_svg(
            "0 0 24 24",
            MARK_DROPBOX,
            bare_glyph,
            rgb(0x0061ff).into(),
        )),
        "mixpanel" => solid_tile(
            rgb(0x7856ff).into(),
            pro_svg("0 0 24 24", MARK_MIXPANEL, glyph, rgb(0xffffff).into()),
            None,
        ),
        "make" => solid_tile(
            rgb(0x6d00cc).into(),
            pro_svg("0 0 24 24", MARK_MAKE, glyph, rgb(0xffffff).into()),
            None,
        ),
        "miro" => solid_tile(
            rgb(0xfddd35).into(),
            pro_svg("0 0 24 24", MARK_MIRO, glyph, rgb(0x171717).into()),
            None,
        ),
        _ => pro_app_tile(name, side, cx),
    }
}

fn default_integrations() -> Vec<IntegrationItem> {
    vec![
        // Popular (6 visible + 4 collapsed)
        IntegrationItem::new(
            "slack",
            "Slack",
            "Get alerts in your channels and act on them without leaving the conversation.",
            IntegrationCategory::Popular,
        )
        .with_verified(true)
        .with_connected(true)
        .with_scopes(vec![
            IntegrationScope::new("channels:read", "List public and shared workspace channels"),
            IntegrationScope::new("chat:write", "Post interactive alerts and thread replies"),
            IntegrationScope::new("commands", "Run slash actions directly from channels"),
        ]),
        IntegrationItem::new(
            "notion",
            "Notion",
            "Turn records into pages and keep project docs in sync both ways.",
            IntegrationCategory::Popular,
        )
        .with_verified(true),
        IntegrationItem::new(
            "linear",
            "Linear",
            "Create issues from customer feedback and track them to release.",
            IntegrationCategory::Popular,
        )
        .with_verified(true),
        IntegrationItem::new(
            "figma",
            "Figma",
            "Preview designs inline and link frames to the work they belong to.",
            IntegrationCategory::Popular,
        )
        .with_verified(true),
        IntegrationItem::new(
            "github",
            "GitHub",
            "Link pull requests and see deploy status next to every task.",
            IntegrationCategory::Popular,
        )
        .with_verified(true),
        IntegrationItem::new(
            "hubspot",
            "HubSpot",
            "Keep contacts, companies and deals matched across both tools.",
            IntegrationCategory::Popular,
        )
        .with_verified(true),
        IntegrationItem::new(
            "stripe",
            "Stripe",
            "Sync subscriptions, invoices and customer billing status automatically.",
            IntegrationCategory::Popular,
        )
        .with_verified(true)
        .with_collapsed(true),
        IntegrationItem::new(
            "zapier",
            "Zapier",
            "Trigger automated workflows across thousands of apps on any update.",
            IntegrationCategory::Popular,
        )
        .with_verified(true)
        .with_collapsed(true),
        IntegrationItem::new(
            "airtable",
            "Airtable",
            "Mirror structured bases and views into your workspace records.",
            IntegrationCategory::Popular,
        )
        .with_collapsed(true),
        IntegrationItem::new(
            "jira",
            "Jira",
            "Link engineering epics and sprint issues to customer feedback.",
            IntegrationCategory::Popular,
        )
        .with_verified(true)
        .with_collapsed(true),
        // Google (3 visible)
        IntegrationItem::new(
            "google-drive",
            "Google Drive",
            "Pick files from Drive and preview them without downloading.",
            IntegrationCategory::Google,
        ),
        IntegrationItem::new(
            "google-calendar",
            "Google Calendar",
            "See upcoming meetings with every contact in one timeline.",
            IntegrationCategory::Google,
        ),
        IntegrationItem::new(
            "google-analytics",
            "Google Analytics",
            "Attribute signups to the campaigns and pages that brought them.",
            IntegrationCategory::Google,
        ),
        // Communication (6 visible + 3 collapsed)
        IntegrationItem::new(
            "zoom",
            "Zoom",
            "Start meetings from any record and save the recording link.",
            IntegrationCategory::Communication,
        ),
        IntegrationItem::new(
            "discord",
            "Discord",
            "Post updates to community channels when something ships.",
            IntegrationCategory::Communication,
        ),
        IntegrationItem::new(
            "intercom",
            "Intercom",
            "See conversations next to the customer and reply from one place.",
            IntegrationCategory::Communication,
        ),
        IntegrationItem::new(
            "calendly",
            "Calendly",
            "Log booked meetings and the answers people gave when booking.",
            IntegrationCategory::Communication,
        ),
        IntegrationItem::new(
            "typeform",
            "Typeform",
            "Create records from form answers the moment they come in.",
            IntegrationCategory::Communication,
        ),
        IntegrationItem::new(
            "dropbox",
            "Dropbox",
            "Attach files from Dropbox and keep shared folders linked.",
            IntegrationCategory::Communication,
        ),
        IntegrationItem::new(
            "mixpanel",
            "Mixpanel",
            "Chart product cohorts and funnel conversion next to accounts.",
            IntegrationCategory::Communication,
        )
        .with_collapsed(true),
        IntegrationItem::new(
            "make",
            "Make",
            "Build multi-step webhook scenarios with visual branching.",
            IntegrationCategory::Communication,
        )
        .with_collapsed(true),
        IntegrationItem::new(
            "miro",
            "Miro",
            "Embed collaborative boards and workshop canvases inline.",
            IntegrationCategory::Communication,
        )
        .with_collapsed(true),
    ]
}

fn filter_integrations(
    items: &[IntegrationItem],
    filter: IntegrationFilter,
    query: &str,
) -> Vec<usize> {
    let q = query.trim().to_ascii_lowercase();
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            let status_ok = match filter {
                IntegrationFilter::All => true,
                IntegrationFilter::Connected => item.connected,
                IntegrationFilter::Available => !item.connected,
            };
            let query_ok = q.is_empty()
                || item.name.to_ascii_lowercase().contains(&q)
                || item.description.to_ascii_lowercase().contains(&q)
                || item.category.as_id().contains(&q);
            status_ok && query_ok
        })
        .map(|(idx, _)| idx)
        .collect()
}

fn format_more_label(names: &[&str], more_word: &str) -> String {
    match names {
        [] => String::new(),
        [a] => (*a).to_string(),
        [a, b] => format!("{a} and {b}"),
        [a, b, rest @ ..] => format!("{a}, {b} and {} {more_word}", rest.len()),
    }
}

/// Interactive app directory screen with categorized integrations, search, and detail drawer.
pub struct IntegrationsScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    loading: bool,
    empty: bool,
    filter: IntegrationFilter,
    items: Vec<IntegrationItem>,
    expanded_categories: [bool; 3],
    selected_id: Option<SharedString>,
    focused_idx: usize,
    request_open: bool,
    search_editor: Option<Entity<Editor>>,
    request_editor: Option<Entity<Editor>>,
    toast: Option<(SharedString, Instant)>,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
    anim_start: Instant,
}

impl IntegrationsScreen {
    /// Construct an [`IntegrationsScreen`] with the default `Acme` workspace and 22 integrations.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Chat, shell_cx)
                .with_workspace("Acme")
                .with_user("Alex Morgan", "alex@acme.ai")
                .with_active(tr(shell_cx, "Integrations", "集成"))
                .with_work_area(false, false)
                .with_panel_width(340.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });

        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(IntegrationsEvent::Navigate(label.clone()));
            }
            AppShellEvent::PanelOpen(false) => {
                this.selected_id = None;
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            standalone: false,
            loading: false,
            empty: false,
            filter: IntegrationFilter::All,
            items: default_integrations(),
            expanded_categories: [false; 3],
            selected_id: None,
            focused_idx: 0,
            request_open: false,
            search_editor: None,
            request_editor: None,
            toast: None,
            focus: cx.focus_handle(),
            measured_width: Rc::new(Cell::new(900.)),
            anim_start: Instant::now(),
        }
    }

    /// Render without the surrounding [`AppShell`] frame.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the workspace title shown in the sidebar.
    pub fn with_workspace(self, workspace: impl Into<SharedString>, cx: &mut App) -> Self {
        let ws: SharedString = workspace.into();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_workspace(ws);
            cx.notify();
        });
        self
    }

    /// Override the signed-in account shown at the bottom of the sidebar.
    pub fn with_user(
        self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut App,
    ) -> Self {
        let name: SharedString = name.into();
        let email: SharedString = email.into();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_user(name, email);
            cx.notify();
        });
        self
    }

    /// Configure the initial loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the initial empty state (`loading` wins when both are enabled).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Set the initial filter tab (`All`, `Connected`, `Available`).
    pub fn with_filter(mut self, filter: IntegrationFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Replace the integration items list.
    pub fn with_items(mut self, items: Vec<IntegrationItem>) -> Self {
        self.items = items;
        self
    }

    /// Toggle the loading skeleton state dynamically.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle the empty state dynamically.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Return whether the loading skeleton state is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Return whether the empty state override is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Return the currently selected status filter tab.
    pub fn filter(&self) -> IntegrationFilter {
        self.filter
    }

    /// Return the current integration items.
    pub fn items(&self) -> &[IntegrationItem] {
        &self.items
    }

    /// Access the underlying [`AppShell`].
    pub fn shell(&self) -> &Entity<AppShell> {
        &self.shell
    }

    fn show_toast(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.notify();
    }

    fn toggle_connection(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        if let Some(item) = self.items.iter_mut().find(|it| &it.id == id) {
            item.connected = !item.connected;
            let connected = item.connected;
            let name = item.name.clone();
            let item_id = item.id.clone();
            let msg = if connected {
                format!("{} {name}", tr(cx, "Connected", "已连接"))
            } else {
                format!("{} {name}", tr(cx, "Disconnected", "已断开"))
            };
            self.show_toast(msg, cx);
            cx.emit(IntegrationsEvent::ConnectionToggled {
                id: item_id,
                connected,
            });
        }
    }

    fn select_integration(&mut self, id: SharedString, cx: &mut Context<Self>) {
        self.selected_id = Some(id.clone());
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(IntegrationsEvent::Selected(id));
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Search integrations", "搜索集成应用").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed | EditEvent::Submitted) {
                    cx.notify();
                }
            })
            .detach();
            self.search_editor = Some(ed);
        }

        if self.request_editor.is_none() {
            let ph: SharedString = tr(cx, "App name or URL", "应用名称或网址").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Submitted => {
                    this.submit_request_without_window(cx);
                }
                EditEvent::Escape => {
                    this.request_open = false;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.request_editor = Some(ed);
        }
    }

    fn submit_request_without_window(&mut self, cx: &mut Context<Self>) {
        let Some(ed) = &self.request_editor else {
            return;
        };
        let val = ed.read(cx).value().trim().to_string();
        if val.is_empty() {
            return;
        }
        self.request_open = false;
        let msg = format!("{}: {val}", tr(cx, "Request submitted", "已提交申请"));
        self.show_toast(msg, cx);
        cx.emit(IntegrationsEvent::IntegrationRequested(val.into()));
    }

    fn submit_request(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.request_editor.clone() else {
            return;
        };
        let val = ed.read(cx).value().trim().to_string();
        if val.is_empty() {
            return;
        }
        ed.update(cx, |e, cx| e.set_value("", w, cx));
        self.request_open = false;
        let msg = format!("{}: {val}", tr(cx, "Request submitted", "已提交申请"));
        self.show_toast(msg, cx);
        cx.emit(IntegrationsEvent::IntegrationRequested(val.into()));
    }

    fn on_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if key == "escape" {
            if self.request_open {
                self.request_open = false;
                cx.notify();
            } else if self.selected_id.is_some() {
                self.selected_id = None;
                self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
                cx.notify();
            }
            return;
        }
        if key == "/" {
            if let Some(ed) = self.search_editor.clone() {
                ed.update(cx, |e, cx| e.focus(w, cx));
            }
            return;
        }

        let query = self
            .search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        let matched = filter_integrations(&self.items, self.filter, &query);
        if matched.is_empty() {
            return;
        }

        match key {
            "down" | "j" => {
                self.focused_idx = (self.focused_idx + 1).min(matched.len() - 1);
                cx.notify();
            }
            "up" | "k" => {
                self.focused_idx = self.focused_idx.saturating_sub(1);
                cx.notify();
            }
            "enter" | "i" => {
                let item_idx = matched[self.focused_idx.min(matched.len() - 1)];
                let id = self.items[item_idx].id.clone();
                self.select_integration(id, cx);
            }
            " " => {
                let item_idx = matched[self.focused_idx.min(matched.len() - 1)];
                let id = self.items[item_idx].id.clone();
                self.toggle_connection(&id, cx);
            }
            _ => {}
        }
    }

    fn render_row(&self, item_idx: usize, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let item = &self.items[item_idx];
        let id = item.id.clone();
        let toggle_id = item.id.clone();
        let connected = item.connected;
        let is_selected = self.selected_id.as_ref() == Some(&id);
        let btn_aria: SharedString = if connected {
            format!("{} {}", tr(cx, "Disconnect", "断开"), item.name).into()
        } else {
            format!("{} {}", tr(cx, "Connect", "连接"), item.name).into()
        };

        let connect_btn = div()
            .id(pro_eid(&self.id, format!("conn-{id}")))
            .role(Role::Button)
            .aria_label(btn_aria)
            .tab_index(0)
            .size(px(28.))
            .flex_none()
            .rounded(px(8.))
            .bg(if connected { p.primary } else { p.card })
            .when(!connected, |d| {
                d.border_1()
                    .border_color(p.line)
                    .hover(move |s| s.border_color(p.line_strong).bg(p.hover))
            })
            .when(connected, |d| {
                d.hover(move |s| s.bg(p.primary.opacity(0.88)))
            })
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, _, _, cx| {
                this.toggle_connection(&toggle_id, cx);
            }))
            .child(pro_icon(
                if connected {
                    ProIcon::Check
                } else {
                    ProIcon::Plus
                },
                15.,
                if connected { p.on_primary } else { p.text },
            ));

        div().mx(px(-8.)).child(activate(
            div()
                .id(pro_eid(&self.id, format!("row-{id}")))
                .role(Role::Button)
                .tab_index(0)
                .h(px(58.))
                .px(px(8.))
                .rounded(px(12.))
                .when(is_selected, |d| d.bg(p.select))
                .when(!is_selected, |d| d.hover(move |s| s.bg(p.subtle)))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(12.))
                .cursor_pointer()
                .child(render_integration_logo(&item.id, &item.name, 30., cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .min_w_0()
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(14.))
                                        .line_height(px(19.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(item.name.clone()),
                                )
                                .when(item.verified, |d| {
                                    d.child(pro_svg("0 0 24 24", ICON_VERIFIED_BADGE, 14., p.faint))
                                }),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(13.))
                                .line_height(px(18.))
                                .text_color(p.muted)
                                .child(item.description.clone()),
                        ),
                )
                .child(connect_btn),
            cx,
            move |this, _, cx| {
                this.select_integration(id.clone(), cx);
            },
        ))
    }

    fn render_grid_pairs(
        &self,
        indices: &[usize],
        single_col: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut grid = div().mt(px(12.)).flex().flex_col();
        if single_col {
            for &idx in indices {
                grid = grid.child(self.render_row(idx, cx));
            }
        } else {
            let mut i = 0usize;
            while i < indices.len() {
                let left = self.render_row(indices[i], cx);
                let right = if i + 1 < indices.len() {
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(self.render_row(indices[i + 1], cx))
                } else {
                    div().flex_1().min_w_0()
                };
                grid = grid.child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(32.))
                        .child(div().flex_1().min_w_0().child(left))
                        .child(right),
                );
                i += 2;
            }
        }
        grid
    }

    fn render_skeleton_sections(&self, single_col: bool, w: &mut Window, cx: &App) -> Div {
        let phase = if Theme::global(cx).reduced_motion {
            0.25
        } else {
            w.request_animation_frame();
            (self.anim_start.elapsed().as_secs_f32() * 0.75).fract()
        };

        let mut wrap = div().flex().flex_col();
        for sec in 0..2usize {
            let mut grid = div().mt(px(12.)).flex().flex_col();
            let row_pairs = if single_col { 6usize } else { 3usize };
            for r in 0..row_pairs {
                let ph_a = (phase + (sec * 6 + r * 2) as f32 * 0.06).fract();
                let ph_b = (phase + (sec * 6 + r * 2 + 1) as f32 * 0.06).fract();
                let skel_item = |ph: f32, name_w: f32, desc_w: f32| {
                    div()
                        .h(px(58.))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(30., 30., 8., ph, cx))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(7.))
                                .child(pro_skeleton_bar(name_w, 12., 5., ph, cx))
                                .child(pro_skeleton_bar(desc_w, 10., 5., ph, cx)),
                        )
                        .child(pro_skeleton_bar(28., 28., 8., ph, cx))
                };

                if single_col {
                    grid = grid.child(skel_item(ph_a, 84., 210.));
                } else {
                    grid = grid.child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap(px(32.))
                            .child(div().flex_1().min_w_0().child(skel_item(ph_a, 78., 208.)))
                            .child(div().flex_1().min_w_0().child(skel_item(ph_b, 92., 196.))),
                    );
                }
            }

            wrap = wrap.child(
                div()
                    .mt(px(if sec == 0 { 38. } else { 46. }))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .child(pro_skeleton_bar(
                                if sec == 0 { 72. } else { 114. },
                                13.,
                                5.,
                                phase,
                                cx,
                            )),
                    )
                    .child(grid),
            );
        }
        wrap
    }

    fn render_inspector(
        &mut self,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let sel_id = self.selected_id.clone()?;
        let item = self.items.iter().find(|it| it.id == sel_id)?.clone();
        let p = ProPalette::get(cx);
        let toggle_id = item.id.clone();
        let action_id = item.id.clone();
        let webhook_id = item.id.clone();
        let scope_owner_id = item.id.clone();

        let body = div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .p(px(12.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(render_integration_logo(&item.id, &item.name, 36., cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(5.))
                                            .child(
                                                div()
                                                    .text_size(px(15.))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(p.strong)
                                                    .truncate()
                                                    .child(item.name.clone()),
                                            )
                                            .when(item.verified, |d| {
                                                d.child(pro_svg(
                                                    "0 0 24 24",
                                                    ICON_VERIFIED_BADGE,
                                                    14.,
                                                    p.blue,
                                                ))
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .child(item.category.title(cx)),
                                    ),
                            )
                            .child(pro_badge(
                                if item.connected {
                                    tr(cx, "Connected", "已连接")
                                } else {
                                    tr(cx, "Available", "未连接")
                                },
                                if item.connected {
                                    ProTone::Green
                                } else {
                                    ProTone::Neutral
                                },
                                item.connected,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.text)
                            .child(item.description.clone()),
                    ),
            )
            .child(
                div()
                    .p(px(12.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(cx, "Workspace connection", "工作区连接开关")),
                            )
                            .child(div().text_size(px(12.)).text_color(p.muted).child(
                                if item.connected {
                                    item.account.clone()
                                } else {
                                    tr(cx, "Authorize OAuth 2.0 access", "使用 OAuth 2.0 授权访问")
                                        .into()
                                },
                            )),
                    )
                    .child(pro_switch(
                        pro_eid(&self.id, format!("insp-sw-{}", item.id)),
                        format!("Toggle {}", item.name),
                        item.connected,
                        cx,
                        move |this, _, cx| {
                            this.toggle_connection(&toggle_id, cx);
                        },
                    )),
            )
            .child(
                div()
                    .p(px(12.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(cx, "Webhook & sync status", "Webhook 与同步状态")),
                            )
                            .child(pro_btn(
                                pro_eid(&self.id, "test-webhook"),
                                tr(cx, "Ping", "测试"),
                                Some(ProIcon::Refresh),
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                move |this, _, cx| {
                                    this.show_toast(
                                        tr(
                                            cx,
                                            "Webhook ping 200 OK (42ms)",
                                            "Webhook 响应正常 (42ms)",
                                        ),
                                        cx,
                                    );
                                    cx.emit(IntegrationsEvent::WebhookTested(webhook_id.clone()));
                                },
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(if item.connected {
                                item.sync_status.clone()
                            } else {
                                tr(cx, "No active webhook deliveries", "连接后自动开启实时同步")
                                    .into()
                            }),
                    ),
            )
            .child({
                let total_sc = item.scopes.len();
                let enabled_sc = item.scopes.iter().filter(|s| s.enabled).count();
                let sc_ratio = if total_sc == 0 {
                    0.0
                } else {
                    enabled_sc as f32 / total_sc as f32
                };
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Permissions & scopes", "权限范围")),
                            )
                            .when(total_sc > 0, |d| {
                                d.child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(p.faint)
                                        .child(format!("{enabled_sc}/{total_sc}")),
                                )
                            }),
                    )
                    .when(total_sc > 0, |d| {
                        d.child(pro_spring_bar(
                            pro_eid(&self.id, format!("scopes-bar-{}", item.id)),
                            sc_ratio,
                            4.,
                            2.,
                            p.chip,
                            if item.connected { p.green } else { p.blue },
                        ))
                    })
                    .children(item.scopes.into_iter().enumerate().map(|(si, sc)| {
                        let sid = scope_owner_id.clone();
                        let skey = sc.key.clone();
                        let skey_label = sc.key.clone();
                        let enabled = sc.enabled;
                        div()
                            .p(px(10.))
                            .rounded(px(9.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(10.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(skey_label),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .line_height(px(16.))
                                            .text_color(p.muted)
                                            .child(sc.description),
                                    ),
                            )
                            .child(pro_switch(
                                pro_eid(&self.id, format!("scope-sw-{si}")),
                                format!("Scope {skey}"),
                                enabled,
                                cx,
                                move |this, _, cx| {
                                    if let Some(it) = this.items.iter_mut().find(|x| x.id == sid)
                                        && let Some(target_sc) =
                                            it.scopes.iter_mut().find(|s| s.key == skey)
                                    {
                                        target_sc.enabled = !target_sc.enabled;
                                        let next_on = target_sc.enabled;
                                        cx.emit(IntegrationsEvent::ScopeToggled {
                                            id: sid.clone(),
                                            scope: skey.clone(),
                                            enabled: next_on,
                                        });
                                        cx.notify();
                                    }
                                },
                            ))
                    }))
            })
            .child(div().pt(px(2.)).child(pro_btn(
                pro_eid(&self.id, "insp-conn-action"),
                if item.connected {
                    format!("{} {}", tr(cx, "Disconnect", "断开连接"), item.name)
                } else {
                    format!("{} {}", tr(cx, "Connect", "立即连接"), item.name)
                },
                Some(if item.connected {
                    ProIcon::Close
                } else {
                    ProIcon::Plus
                }),
                if item.connected {
                    ProBtnStyle::Danger
                } else {
                    ProBtnStyle::Primary
                },
                false,
                cx,
                move |this, _, cx| {
                    this.toggle_connection(&action_id, cx);
                },
            )));

        let animated_body = pro_view_transition(pro_eid(&self.id, "int-insp-vtx"), sel_id, body);
        Some((item.name, ProIcon::Grid, animated_body))
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let width = self.measured_width.get();
        let single_col = width < 700.;
        let compact = width < 560.;
        let measure_cell = self.measured_width.clone();

        if let Some((_, at)) = &self.toast
            && at.elapsed().as_millis() > 2450
        {
            self.toast = None;
        }

        let query = self
            .search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        let has_query = !query.trim().is_empty();

        // Filter tabs: `All`, `Connected`, `Available`
        let cur_filter = self.filter;
        let flt_sensor = pro_selection_sensor();
        let tabs_el = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(4.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "int-flt-pill"),
                cur_filter.as_id(),
                flt_sensor.clone(),
                7.,
                p.chip,
                Some(p.line),
                false,
            ))
            .children(IntegrationFilter::ALL.into_iter().map(|flt| {
                let selected = flt == cur_filter;
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("tab-{}", flt.as_id())))
                        .role(Role::Tab)
                        .tab_index(0)
                        .relative()
                        .h(px(28.))
                        .px(px(10.))
                        .rounded(px(7.))
                        .when(selected, |d| d.child(pro_mark_selected(&flt_sensor)))
                        .when(!selected, |d| {
                            d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                        })
                        .flex()
                        .items_center()
                        .text_size(px(13.))
                        .font_weight(if selected {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if selected { p.ink } else { p.text })
                        .whitespace_nowrap()
                        .cursor_pointer()
                        .child(flt.label(cx)),
                    cx,
                    move |this, _, cx| {
                        this.filter = flt;
                        this.focused_idx = 0;
                        cx.emit(IntegrationsEvent::FilterChanged(flt));
                        cx.notify();
                    },
                )
            }));

        let search_ed = self.search_editor.clone();
        let search_box = div()
            .h(px(28.))
            .w(px(if compact { 164. } else { 208. }))
            .px(px(9.))
            .rounded(px(6.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(6.))
            .child(pro_icon(ProIcon::Search, 14., p.muted))
            .when_some(search_ed.clone(), |d, ed| {
                d.child(div().flex_1().min_w_0().child(ed))
            })
            .when(has_query, |d| {
                let clear_ed = search_ed.clone();
                d.child(activate(
                    div()
                        .id(pro_eid(&self.id, "clear-search"))
                        .role(Role::Button)
                        .tab_index(0)
                        .size(px(16.))
                        .rounded(px(4.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .child(pro_icon(ProIcon::Close, 11., p.muted)),
                    cx,
                    move |_, w, cx| {
                        if let Some(ed) = &clear_ed {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                    },
                ))
            });

        let toolbar = div()
            .mt(px(24.))
            .py(px(6.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(tabs_el)
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(search_box)
                    .child(pro_btn(
                        pro_eid(&self.id, "req-btn"),
                        tr(cx, "Request", "申请集成"),
                        Some(ProIcon::Plus),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, w, cx| {
                            this.request_open = !this.request_open;
                            if this.request_open
                                && let Some(ed) = this.request_editor.clone()
                            {
                                ed.update(cx, |e, cx| e.focus(w, cx));
                            }
                            cx.notify();
                        },
                    )),
            );

        // Request integration inline/popover form with spring disclosure & settle
        let req_dialog_box = div()
            .id(pro_eid(&self.id, "request-popover"))
            .role(Role::Dialog)
            .w(px(276.))
            .p(px(14.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_md()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(tr(cx, "Request an integration", "申请新集成")),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .line_height(px(17.))
                    .text_color(p.muted)
                    .child(tr(
                        cx,
                        "Tell us which tool your team wants to connect next.",
                        "告诉我们您的团队希望接入哪款工具。",
                    )),
            )
            .child(
                div()
                    .h(px(30.))
                    .px(px(9.))
                    .rounded(px(7.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .when_some(self.request_editor.clone(), |d, ed| {
                        d.child(div().flex_1().min_w_0().child(ed))
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(6.))
                    .child(pro_btn(
                        pro_eid(&self.id, "req-cancel"),
                        tr(cx, "Cancel", "取消"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, _, cx| {
                            this.request_open = false;
                            cx.notify();
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "req-submit"),
                        tr(cx, "Send request", "提交申请"),
                        None,
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, w, cx| this.submit_request(w, cx),
                    )),
            );
        let req_pres = pro_presence(
            pro_eid(&self.id, "req-pres"),
            self.request_open,
            0.20,
            0.14,
            move |a| {
                div()
                    .mt(px(8.))
                    .w_full()
                    .flex()
                    .justify_end()
                    .child(req_dialog_box.relative().top(px(-4. * (1. - a))).opacity(a))
                    .into_any_element()
            },
        );
        let request_fold = pro_fold(
            pro_eid(&self.id, "req-fold"),
            self.request_open,
            168.,
            0.,
            move || req_pres,
        );

        let matched = filter_integrations(&self.items, self.filter, &query);

        // Content body: loading wins over empty per spec!
        let content_el: AnyElement = if self.loading {
            self.render_skeleton_sections(single_col, w, cx)
                .into_any_element()
        } else if self.empty || self.items.is_empty() {
            div()
                .mt(px(28.))
                .child(pro_empty_state(
                    ProIcon::Grid,
                    tr(cx, "No integrations yet", "暂无可用集成"),
                    tr(
                        cx,
                        "Request the tools your team relies on and we will notify you when they ship.",
                        "提交团队常用的工具需求，上线后我们将第一时间通知您。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-req"),
                        tr(cx, "Request integration", "申请集成").into(),
                        Box::new(|this: &mut Self, w, cx| {
                            this.request_open = true;
                            if let Some(ed) = this.request_editor.clone() {
                                ed.update(cx, |e, cx| e.focus(w, cx));
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else if matched.is_empty() {
            div()
                .mt(px(28.))
                .child(pro_empty_state(
                    ProIcon::Search,
                    tr(cx, "No matching integrations", "未找到匹配的集成"),
                    tr(
                        cx,
                        "Try another search term, switch tabs, or request a new connector.",
                        "请可改其他关键词、切换筛选标签或直接申请新连接器。",
                    ),
                    Some((
                        pro_eid(&self.id, "clear-empty-flt"),
                        tr(cx, "Show all integrations", "显示全部集成").into(),
                        Box::new(|this: &mut Self, w, cx| {
                            this.filter = IntegrationFilter::All;
                            if let Some(ed) = &this.search_editor {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else {
            let allow_collapse = self.filter == IntegrationFilter::All && !has_query;
            let mut sections_col = div().flex().flex_col();
            let mut rendered_any = false;

            for (cat_idx, cat) in IntegrationCategory::ALL.into_iter().enumerate() {
                let sec_all: Vec<usize> = matched
                    .iter()
                    .copied()
                    .filter(|&idx| self.items[idx].category == cat)
                    .collect();
                if sec_all.is_empty() {
                    continue;
                }

                let expanded = self.expanded_categories[cat_idx] || !allow_collapse;
                let base_indices: Vec<usize> = if allow_collapse {
                    sec_all
                        .iter()
                        .copied()
                        .filter(|&idx| !self.items[idx].collapsed_by_default)
                        .collect()
                } else {
                    sec_all.clone()
                };
                let hidden_indices: Vec<usize> = if allow_collapse {
                    sec_all
                        .iter()
                        .copied()
                        .filter(|&idx| self.items[idx].collapsed_by_default)
                        .collect()
                } else {
                    Vec::new()
                };

                let top_margin = if !rendered_any { 38. } else { 46. };
                rendered_any = true;

                let mut section = div()
                    .mt(px(top_margin))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(cat.title(cx)),
                    )
                    .child(self.render_grid_pairs(&base_indices, single_col, cx));

                if allow_collapse && !hidden_indices.is_empty() {
                    let hidden_rows = if single_col {
                        hidden_indices.len()
                    } else {
                        hidden_indices.len().div_ceil(2)
                    };
                    let overflow_grid = self
                        .render_grid_pairs(&hidden_indices, single_col, cx)
                        .mt(px(0.));
                    section = section.child(pro_fold(
                        pro_eid(&self.id, format!("cat-more-fold-{}", cat.as_id())),
                        expanded,
                        (hidden_rows * 58) as f32,
                        0.,
                        move || overflow_grid.into_any_element(),
                    ));

                    if !expanded {
                        let names: Vec<&str> = hidden_indices
                            .iter()
                            .map(|&idx| self.items[idx].name.as_ref())
                            .collect();
                        let summary_text = format_more_label(&names, tr(cx, "more", "项更多"));
                        let preview_logos: Vec<AnyElement> = hidden_indices
                            .iter()
                            .take(3)
                            .map(|&idx| {
                                render_integration_logo(
                                    &self.items[idx].id,
                                    &self.items[idx].name,
                                    20.,
                                    cx,
                                )
                            })
                            .collect();

                        section = section.child(
                            div().mt(px(8.)).flex().child(activate(
                                div()
                                    .id(pro_eid(&self.id, format!("more-{}", cat.as_id())))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .py(px(4.))
                                    .pl(px(4.))
                                    .pr(px(8.))
                                    .rounded(px(10.))
                                    .hover(move |s| s.bg(p.chip))
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .text_size(px(13.))
                                    .text_color(p.text)
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .p(px(3.))
                                            .rounded(px(9.))
                                            .bg(p.card)
                                            .border_1()
                                            .border_color(p.line)
                                            .flex()
                                            .items_center()
                                            .gap(px(3.))
                                            .children(preview_logos),
                                    )
                                    .child(summary_text)
                                    .child(pro_icon(ProIcon::ChevronRight, 14., p.faint)),
                                cx,
                                move |this, _, cx| {
                                    this.expanded_categories[cat_idx] = true;
                                    cx.notify();
                                },
                            )),
                        );
                    } else {
                        section = section.child(
                            div().mt(px(8.)).flex().child(activate(
                                div()
                                    .id(pro_eid(&self.id, format!("less-{}", cat.as_id())))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .h(px(28.))
                                    .px(px(8.))
                                    .rounded(px(8.))
                                    .hover(move |s| s.bg(p.chip))
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(12.5))
                                    .text_color(p.muted)
                                    .cursor_pointer()
                                    .child(tr(cx, "Show fewer", "收起更多"))
                                    .child(pro_icon(ProIcon::ChevronUp, 13., p.faint)),
                                cx,
                                move |this, _, cx| {
                                    this.expanded_categories[cat_idx] = false;
                                    cx.notify();
                                },
                            )),
                        );
                    }
                }

                sections_col = sections_col.child(section);
            }

            sections_col.into_any_element()
        };

        let dir_vtx = pro_view_transition(
            pro_eid(&self.id, "int-dir-vtx"),
            format!("{}-{}-{}", self.filter.as_id(), self.loading, self.empty),
            content_el,
        );

        div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .relative()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10. && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .size_0(),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scroll()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(1021.))
                            .px(px(if compact { 16. } else { 32. }))
                            .pt(px(40.))
                            .pb(px(64.))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(if compact { 20. } else { 24. }))
                                    .line_height(px(if compact { 26. } else { 32. }))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Integrations", "集成")),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(p.text)
                                    .child(tr(
                                        cx,
                                        "Connect the tools your team already uses and work with them right in the chat.",
                                        "连接团队已经在使用的工具，并直接在对话中协同处理。",
                                    )),
                            )
                            .child(toolbar)
                            .child(request_fold)
                            .child(dir_vtx),
                    ),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

impl Render for IntegrationsScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        IntegrationCategory, IntegrationFilter, default_integrations, filter_integrations,
        format_more_label,
    };

    #[test]
    fn filter_by_status_and_search_query() {
        let items = default_integrations();
        assert_eq!(items.len(), 22);

        let connected = filter_integrations(&items, IntegrationFilter::Connected, "");
        assert_eq!(connected.len(), 1);
        assert_eq!(items[connected[0]].id.as_ref(), "slack");

        let available = filter_integrations(&items, IntegrationFilter::Available, "");
        assert_eq!(available.len(), 21);

        let pull_requests = filter_integrations(&items, IntegrationFilter::All, "pull requests");
        assert_eq!(pull_requests.len(), 1);
        assert_eq!(items[pull_requests[0]].id.as_ref(), "github");
    }

    #[test]
    fn collapsed_section_summary_formatting() {
        let items = default_integrations();
        let popular_hidden: Vec<&str> = items
            .iter()
            .filter(|i| i.category == IntegrationCategory::Popular && i.collapsed_by_default)
            .map(|i| i.name.as_ref())
            .collect();
        assert_eq!(
            format_more_label(&popular_hidden, "more"),
            "Stripe, Zapier and 2 more"
        );

        let comm_hidden: Vec<&str> = items
            .iter()
            .filter(|i| i.category == IntegrationCategory::Communication && i.collapsed_by_default)
            .map(|i| i.name.as_ref())
            .collect();
        assert_eq!(
            format_more_label(&comm_hidden, "more"),
            "Mixpanel, Make and 1 more"
        );
    }
}
