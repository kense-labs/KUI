//! Company and pipeline workspace with open view tabs, editable spreadsheet, and deal inspector.
//!
//! Renders an [`AppShell`] CRM workspace with saved-view breadcrumbs, view tabs
//! (`Company CRM`, `Deals`, `Contacts`, `Renewals`), inline spreadsheet editing with
//! undo/redo history, pipeline board mode, KPI summary metrics, loading skeletons,
//! per-view empty states, and a right-hand account and deal inspector.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, ProTone, pro_app_tile, pro_badge, pro_btn, pro_eid,
            pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_person_avatar,
            pro_pill_indicator, pro_presence, pro_selection_sensor, pro_skeleton_bar,
            pro_spring_bar, pro_view_transition, tr,
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

// Screen-specific Hugeicons Free (MIT) & brand mark inner SVG paths from reference spec.
const ICON_NEW_VIEW: &str = r#"<path d="M4 12.0005L4 14.5446C4 17.7896 4 19.4122 4.88607 20.5111C5.06508 20.7331 5.26731 20.9354 5.48933 21.1144C6.58831 22.0005 8.21082 22.0005 11.4558 22.0005C12.1614 22.0005 12.5141 22.0005 12.8372 21.8865C12.9044 21.8627 12.9702 21.8355 13.0345 21.8047C13.3436 21.6569 13.593 21.4075 14.0919 20.9086L18.8284 16.172C19.4065 15.594 19.6955 15.3049 19.8478 14.9374C20 14.5699 20 14.1611 20 13.3436V10.0005C20 6.22922 20 4.34361 18.8284 3.17203C17.7693 2.11287 16.1265 2.01125 13.0345 2.0015M13 21.5005V21.0005C13 18.172 13 16.7578 13.8787 15.8791C14.7574 15.0005 16.1716 15.0005 19 15.0005H19.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12 5.99954H4M8 1.99954V9.99954" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const ICON_ALL_VIEWS: &str = r#"<path d="M3.88884 9.66294C4.39329 10 5.09552 10 6.49998 10C7.90445 10 8.60668 10 9.11113 9.66294C9.32951 9.51702 9.51701 9.32952 9.66292 9.11114C9.99998 8.60669 9.99998 7.90446 9.99998 6.5C9.99998 5.09554 9.99998 4.39331 9.66292 3.88886C9.51701 3.67048 9.32951 3.48298 9.11113 3.33706C8.60668 3 7.90445 3 6.49998 3C5.09552 3 4.39329 3 3.88884 3.33706C3.67046 3.48298 3.48296 3.67048 3.33705 3.88886C2.99998 4.39331 2.99998 5.09554 2.99998 6.5C2.99998 7.90446 2.99998 8.60669 3.33705 9.11114C3.48296 9.32952 3.67046 9.51702 3.88884 9.66294Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M14.8888 9.66294C15.3933 10 16.0955 10 17.5 10C18.9044 10 19.6067 10 20.1111 9.66294C20.3295 9.51702 20.517 9.32952 20.6629 9.11114C21 8.60669 21 7.90446 21 6.5C21 5.09554 21 4.39331 20.6629 3.88886C20.517 3.67048 20.3295 3.48298 20.1111 3.33706C19.6067 3 18.9044 3 17.5 3C16.0955 3 15.3933 3 14.8888 3.33706C14.6705 3.48298 14.483 3.67048 14.337 3.88886C14 4.39331 14 5.09554 14 6.5C14 7.90446 14 8.60669 14.337 9.11114C14.483 9.32952 14.6705 9.51702 14.8888 9.66294Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M3.88884 20.6629C4.39329 21 5.09552 21 6.49998 21C7.90445 21 8.60668 21 9.11113 20.6629C9.32951 20.517 9.51701 20.3295 9.66292 20.1111C9.99998 19.6067 9.99998 18.9045 9.99998 17.5C9.99998 16.0955 9.99998 15.3933 9.66292 14.8889C9.51701 14.6705 9.32951 14.483 9.11113 14.3371C8.60668 14 7.90445 14 6.49998 14C5.09552 14 4.39329 14 3.88884 14.3371C3.67046 14.483 3.48296 14.6705 3.33705 14.8889C2.99998 15.3933 2.99998 16.0955 2.99998 17.5C2.99998 18.9045 2.99998 19.6067 3.33705 20.1111C3.48296 20.3295 3.67046 20.517 3.88884 20.6629Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M14.8888 20.6629C15.3933 21 16.0955 21 17.5 21C18.9044 21 19.6067 21 20.1111 20.6629C20.3295 20.517 20.517 20.3295 20.6629 20.1111C21 19.6067 21 18.9045 21 17.5C21 16.0955 21 15.3933 20.6629 14.8889C20.517 14.6705 20.3295 14.483 20.1111 14.3371C19.6067 14 18.9044 14 17.5 14C16.0955 14 15.3933 14 14.8888 14.3371C14.6705 14.483 14.483 14.6705 14.337 14.8889C14 15.3933 14 16.0955 14 17.5C14 18.9045 14 19.6067 14.337 20.1111C14.483 20.3295 14.6705 20.517 14.8888 20.6629Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const ICON_COL_TEXT: &str = r#"<path d="M3 3H21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M3 9H11" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M3 15H21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M3 21H11" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const ICON_COL_TAG: &str = r#"<circle cx="17.5" cy="6.5" r="1.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></circle><path d="M2.77423 11.1439C1.77108 12.2643 1.7495 13.9546 2.67016 15.1437C4.49711 17.5033 6.49674 19.5029 8.85633 21.3298C10.0454 22.2505 11.7357 22.2289 12.8561 21.2258C15.8979 18.5022 18.6835 15.6559 21.3719 12.5279C21.6377 12.2187 21.8039 11.8397 21.8412 11.4336C22.0062 9.63798 22.3452 4.46467 20.9403 3.05974C19.5353 1.65481 14.362 1.99377 12.5664 2.15876C12.1603 2.19608 11.7813 2.36233 11.472 2.62811C8.34412 5.31646 5.49781 8.10211 2.77423 11.1439Z" stroke="currentColor" stroke-width="1.9"></path><path d="M7.00002 14.0002L10 17.0002" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const ICON_COL_STAGE: &str = r#"<path d="M12 21C15.7497 21 17.6246 21 18.9389 20.0451C19.3634 19.7367 19.7367 19.3634 20.0451 18.9389C21 17.6246 21 15.7497 21 12C21 8.25027 21 6.3754 20.0451 5.06107C19.7367 4.6366 19.3634 4.26331 18.9389 3.95491C17.6246 3 15.7497 3 12 3C8.25027 3 6.3754 3 5.06107 3.95491C4.6366 4.26331 4.26331 4.6366 3.95491 5.06107C3 6.3754 3 8.25027 3 12C3 15.7497 3 17.6246 3.95491 18.9389C4.26331 19.3634 4.6366 19.7367 5.06107 20.0451C6.3754 21 8.25027 21 12 21Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12 7V11M17 7V17M7 7V14" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const ICON_STAR_FILLED: &str = r#"<path fill="currentColor" d="M13.7276 3.44418L15.4874 6.99288C15.7274 7.48687 16.3673 7.9607 16.9073 8.05143L20.0969 8.58575C22.1367 8.92853 22.6167 10.4206 21.1468 11.8925L18.6671 14.3927C18.2471 14.8161 18.0172 15.6327 18.1471 16.2175L18.8571 19.3125C19.417 21.7623 18.1271 22.71 15.9774 21.4296L12.9877 19.6452C12.4478 19.3226 11.5579 19.3226 11.0079 19.6452L8.01827 21.4296C5.8785 22.71 4.57865 21.7522 5.13859 19.3125L5.84851 16.2175C5.97849 15.6327 5.74852 14.8161 5.32856 14.3927L2.84884 11.8925C1.389 10.4206 1.85895 8.92853 3.89872 8.58575L7.08837 8.05143C7.61831 7.9607 8.25824 7.48687 8.49821 6.99288L10.258 3.44418C11.2179 1.51861 12.7777 1.51861 13.7276 3.44418Z"></path>"#;

const ICON_REDO: &str = r#"<path d="M13 6H8.5C6.01472 6 4 8.01472 4 10.5C4 12.9853 6.01472 15 8.5 15H20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17 12C17 12 19.9999 14.2095 20 15C20.0001 15.7906 17 18 17 18" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const MARK_HUBSPOT: &str = r#"<path fill="currentColor" d="M18.164 7.93V5.084a2.198 2.198 0 001.267-1.978v-.067A2.2 2.2 0 0017.238.845h-.067a2.2 2.2 0 00-2.193 2.193v.067a2.196 2.196 0 001.252 1.973l.013.006v2.852a6.22 6.22 0 00-2.969 1.31l.012-.01-7.828-6.095A2.497 2.497 0 104.3 4.656l-.012.006 7.697 5.991a6.176 6.176 0 00-1.038 3.446c0 1.343.425 2.588 1.147 3.607l-.013-.02-2.342 2.343a1.968 1.968 0 00-.58-.095h-.002a2.033 2.033 0 102.033 2.033 1.978 1.978 0 00-.1-.595l.005.014 2.317-2.317a6.247 6.247 0 104.782-11.134l-.036-.005zm-.964 9.378a3.206 3.206 0 113.215-3.207v.002a3.206 3.206 0 01-3.207 3.207z"></path>"#;

const MARK_STRIPE: &str = r#"<path fill="currentColor" d="M13.976 9.15c-2.172-.806-3.356-1.426-3.356-2.409 0-.831.683-1.305 1.901-1.305 2.227 0 4.515.858 6.09 1.631l.89-5.494C18.252.975 15.697 0 12.165 0 9.667 0 7.589.654 6.104 1.872 4.56 3.147 3.757 4.992 3.757 7.218c0 4.039 2.467 5.76 6.476 7.219 2.585.92 3.445 1.574 3.445 2.583 0 .98-.84 1.545-2.354 1.545-1.875 0-4.965-.921-6.99-2.109l-.9 5.555C5.175 22.99 8.385 24 11.714 24c2.641 0 4.843-.624 6.328-1.813 1.664-1.305 2.525-3.236 2.525-5.732 0-4.128-2.524-5.851-6.594-7.305h.003z"></path>"#;

const MARK_AIRTABLE: &str = r#"<path fill="currentColor" d="M11.992 1.966c-.434 0-.87.086-1.28.257L1.779 5.917c-.503.208-.49.908.012 1.116l8.982 3.558a3.266 3.266 0 0 0 2.454 0l8.982-3.558c.503-.196.503-.908.012-1.116l-8.957-3.694a3.255 3.255 0 0 0-1.272-.257zM23.4 8.056a.589.589 0 0 0-.222.045l-10.012 3.877a.612.612 0 0 0-.38.564v8.896a.6.6 0 0 0 .821.552L23.62 18.1a.583.583 0 0 0 .38-.551V8.653a.6.6 0 0 0-.6-.596zM.676 8.095a.644.644 0 0 0-.48.19C.086 8.396 0 8.53 0 8.69v8.355c0 .442.515.737.908.54l6.27-3.006.307-.147 2.969-1.436c.466-.22.43-.908-.061-1.092L.883 8.138a.57.57 0 0 0-.207-.044z"></path>"#;

const MARK_NOTION: &str = r#"<path fill="currentColor" d="M4.459 4.208c.746.606 1.026.56 2.428.466l13.215-.793c.28 0 .047-.28-.046-.326L17.86 1.968c-.42-.326-.981-.7-2.055-.607L3.01 2.295c-.466.046-.56.28-.374.466zm.793 3.08v13.904c0 .747.373 1.027 1.214.98l14.523-.84c.841-.046.935-.56.935-1.167V6.354c0-.606-.233-.933-.748-.887l-15.177.887c-.56.047-.747.327-.747.933zm14.337.745c.093.42 0 .84-.42.888l-.7.14v10.264c-.608.327-1.168.514-1.635.514-.748 0-.935-.234-1.495-.933l-4.577-7.186v6.952L12.21 19s0 .84-1.168.84l-3.222.186c-.093-.186 0-.653.327-.746l.84-.233V9.854L7.822 9.76c-.094-.42.14-1.026.793-1.073l3.456-.233 4.764 7.279v-6.44l-1.215-.139c-.093-.514.28-.887.747-.933zM1.936 1.035l13.31-.98c1.634-.14 2.055-.047 3.082.7l4.249 2.986c.7.513.934.653.934 1.213v16.378c0 1.026-.373 1.634-1.68 1.726l-15.458.934c-.98.047-1.448-.093-1.962-.747l-3.129-4.06c-.56-.747-.793-1.306-.793-1.96V2.667c0-.839.374-1.54 1.447-1.632z"></path>"#;

const MARK_LINEAR: &str = r#"<path fill="currentColor" d="M2.886 4.18A11.982 11.982 0 0 1 11.99 0C18.624 0 24 5.376 24 12.009c0 3.64-1.62 6.903-4.18 9.105L2.887 4.18ZM1.817 5.626l16.556 16.556c-.524.33-1.075.62-1.65.866L.951 7.277c.247-.575.537-1.126.866-1.65ZM.322 9.163l14.515 14.515c-.71.172-1.443.282-2.195.322L0 11.358a12 12 0 0 1 .322-2.195Zm-.17 4.862 9.823 9.824a12.02 12.02 0 0 1-9.824-9.824Z"></path>"#;

const MARK_FIGMA: &str = r#"<path fill="currentColor" d="M15.852 8.981h-4.588V0h4.588c2.476 0 4.49 2.014 4.49 4.49s-2.014 4.491-4.49 4.491zM12.735 7.51h3.117c1.665 0 3.019-1.355 3.019-3.019s-1.355-3.019-3.019-3.019h-3.117V7.51zm0 1.471H8.148c-2.476 0-4.49-2.014-4.49-4.49S5.672 0 8.148 0h4.588v8.981zm-4.587-7.51c-1.665 0-3.019 1.355-3.019 3.019s1.354 3.02 3.019 3.02h3.117V1.471H8.148zm4.587 15.019H8.148c-2.476 0-4.49-2.014-4.49-4.49s2.014-4.49 4.49-4.49h4.588v8.98zM8.148 8.981c-1.665 0-3.019 1.355-3.019 3.019s1.355 3.019 3.019 3.019h3.117V8.981H8.148zM8.172 24c-2.489 0-4.515-2.014-4.515-4.49s2.014-4.49 4.49-4.49h4.588v4.441c0 2.503-2.047 4.539-4.563 4.539zm-.024-7.51a3.023 3.023 0 0 0-3.019 3.019c0 1.665 1.365 3.019 3.044 3.019 1.705 0 3.093-1.376 3.093-3.068v-2.97H8.148zm7.704 0h-.098c-2.476 0-4.49-2.014-4.49-4.49s2.014-4.49 4.49-4.49h.098c2.476 0 4.49 2.014 4.49 4.49s-2.014 4.49-4.49 4.49zm-.097-7.509c-1.665 0-3.019 1.355-3.019 3.019s1.355 3.019 3.019 3.019h.098c1.665 0 3.019-1.355 3.019-3.019s-1.355-3.019-3.019-3.019h-.098z"></path>"#;

const MARK_SHOPIFY: &str = r#"<path fill="currentColor" d="M15.337 23.979l7.216-1.561s-2.604-17.613-2.625-17.73c-.018-.116-.114-.192-.211-.192s-1.929-.136-1.929-.136-1.275-1.274-1.439-1.411c-.045-.037-.075-.057-.121-.074l-.914 21.104h.023zM11.71 11.305s-.81-.424-1.774-.424c-1.447 0-1.504.906-1.504 1.141 0 1.232 3.24 1.715 3.24 4.629 0 2.295-1.44 3.76-3.406 3.76-2.354 0-3.54-1.465-3.54-1.465l.646-2.086s1.245 1.066 2.28 1.066c.675 0 .975-.545.975-.932 0-1.619-2.654-1.694-2.654-4.359-.034-2.237 1.571-4.416 4.827-4.416 1.257 0 1.875.361 1.875.361l-.945 2.715-.02.01zM11.17.83c.136 0 .271.038.405.135-.984.465-2.064 1.639-2.508 3.992-.656.213-1.293.405-1.889.578C7.697 3.75 8.951.84 11.17.84V.83zm1.235 2.949v.135c-.754.232-1.583.484-2.394.736.466-1.777 1.333-2.645 2.085-2.971.193.501.309 1.176.309 2.1zm.539-2.234c.694.074 1.141.867 1.429 1.755-.349.114-.735.231-1.158.366v-.252c0-.752-.096-1.371-.271-1.871v.002zm2.992 1.289c-.02 0-.06.021-.078.021s-.289.075-.714.21c-.423-1.233-1.176-2.37-2.508-2.37h-.115C12.135.209 11.669 0 11.265 0 8.159 0 6.675 3.877 6.21 5.846c-1.194.365-2.063.636-2.16.674-.675.213-.694.232-.772.87-.075.462-1.83 14.063-1.83 14.063L15.009 24l.927-21.166z"></path>"#;

const MARK_VERCEL: &str = r#"<path fill="currentColor" d="m12 1.608 12 20.784H0Z"></path>"#;

const MARK_CLICKUP: &str = r#"<path fill="currentColor" d="M2 18.439l3.69-2.828c1.961 2.56 4.044 3.739 6.363 3.739 2.307 0 4.33-1.166 6.203-3.704L22 18.405C19.298 22.065 15.941 24 12.053 24 8.178 24 4.788 22.078 2 18.439zM12.04 6.15l-6.568 5.66-3.036-3.52L12.055 0l9.543 8.296-3.05 3.509z"></path>"#;

const MARK_WEBFLOW: &str = r#"<path fill="currentColor" d="m24 4.515-7.658 14.97H9.149l3.205-6.204h-.144C9.566 16.713 5.621 18.973 0 19.485v-6.118s3.596-.213 5.71-2.435H0V4.515h6.417v5.278l.144-.001 2.622-5.277h4.854v5.244h.144l2.72-5.244H24Z"></path>"#;

/// Open view tab inside [`CrmScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CrmView {
    /// Editable company directory and pipeline stage table.
    #[default]
    Companies,
    /// Revenue pipeline deals with KPI strip, stage filters, and board switcher.
    Deals,
    /// Decision-makers and primary contacts across accounts.
    Contacts,
    /// Upcoming contract renewals and account health.
    Renewals,
}

impl CrmView {
    /// Ordered view tabs shown in the CRM view bar.
    pub const ALL: [Self; 4] = [Self::Companies, Self::Deals, Self::Contacts, Self::Renewals];

    /// Stable identifier passed to events and empty-state callbacks.
    pub fn as_id(self) -> &'static str {
        match self {
            Self::Companies => "companies",
            Self::Deals => "deals",
            Self::Contacts => "contacts",
            Self::Renewals => "renewals",
        }
    }

    /// Localized tab label.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Companies => tr(cx, "Company CRM", "客户台账"),
            Self::Deals => tr(cx, "Deals", "商机"),
            Self::Contacts => tr(cx, "Contacts", "联系人"),
            Self::Renewals => tr(cx, "Renewals", "续约"),
        }
    }
}

/// Presentation layout for [`CrmScreen`] records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrmDisplayMode {
    /// Editable spreadsheet rows under sortable column headers.
    #[default]
    Table,
    /// Stage-grouped pipeline board cards.
    Board,
}

/// Pipeline stage for a CRM company or deal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CrmStage {
    #[default]
    New,
    Qualified,
    Proposal,
    Negotiation,
    Won,
    Lost,
}

impl CrmStage {
    /// Canonical progression of CRM stages.
    pub const ALL: [Self; 6] = [
        Self::New,
        Self::Qualified,
        Self::Proposal,
        Self::Negotiation,
        Self::Won,
        Self::Lost,
    ];

    /// English or localized stage label.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::New => tr(cx, "New", "新建"),
            Self::Qualified => tr(cx, "Qualified", "已验证"),
            Self::Proposal => tr(cx, "Proposal", "方案报价"),
            Self::Negotiation => tr(cx, "Negotiation", "商务谈判"),
            Self::Won => tr(cx, "Won", "已赢单"),
            Self::Lost => tr(cx, "Lost", "已输单"),
        }
    }

    /// Canonical English stage string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::New => "New",
            Self::Qualified => "Qualified",
            Self::Proposal => "Proposal",
            Self::Negotiation => "Negotiation",
            Self::Won => "Won",
            Self::Lost => "Lost",
        }
    }

    /// Forecast win probability percentage (`0..=100`).
    pub fn probability(self) -> u8 {
        match self {
            Self::New => 15,
            Self::Qualified => 35,
            Self::Proposal => 60,
            Self::Negotiation => 80,
            Self::Won => 100,
            Self::Lost => 0,
        }
    }

    /// Parse a case-insensitive stage name.
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "new" | "lead" => Some(Self::New),
            "qualified" => Some(Self::Qualified),
            "proposal" => Some(Self::Proposal),
            "negotiation" => Some(Self::Negotiation),
            "won" | "closed won" => Some(Self::Won),
            "lost" | "closed lost" => Some(Self::Lost),
            _ => None,
        }
    }

    fn tag_tone(self) -> TagTone {
        match self {
            Self::New => TagTone::Gray,
            Self::Qualified => TagTone::Blue,
            Self::Proposal => TagTone::Violet,
            Self::Negotiation => TagTone::Amber,
            Self::Won => TagTone::Green,
            Self::Lost => TagTone::Red,
        }
    }

    fn sort_rank(self) -> u8 {
        match self {
            Self::New => 0,
            Self::Qualified => 1,
            Self::Proposal => 2,
            Self::Negotiation => 3,
            Self::Won => 4,
            Self::Lost => 5,
        }
    }
}

/// Stage filter applied to [`CrmScreen`] views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrmStageFilter {
    #[default]
    All,
    Stage(CrmStage),
}

/// One logged activity or note entry on a [`CrmCompany`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrmActivityNote {
    pub author: SharedString,
    pub summary: SharedString,
    pub timestamp: SharedString,
}

impl CrmActivityNote {
    /// Construct a timeline note entry.
    pub fn new(
        author: impl Into<SharedString>,
        summary: impl Into<SharedString>,
        timestamp: impl Into<SharedString>,
    ) -> Self {
        Self {
            author: author.into(),
            summary: summary.into(),
            timestamp: timestamp.into(),
        }
    }
}

/// Company, deal, contact, and renewal record backing [`CrmScreen`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrmCompany {
    pub id: SharedString,
    pub name: SharedString,
    pub category: SharedString,
    pub domain: SharedString,
    pub owner: SharedString,
    pub value: u64,
    pub stage: CrmStage,
    pub contact: SharedString,
    pub contact_email: SharedString,
    pub contact_role: SharedString,
    pub last_contact: SharedString,
    pub next_step: SharedString,
    pub renewal_date: SharedString,
    pub notes: Vec<CrmActivityNote>,
}

impl CrmCompany {
    /// Create a CRM company record with sensible contact and renewal defaults.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        category: impl Into<SharedString>,
        domain: impl Into<SharedString>,
        owner: impl Into<SharedString>,
        value: u64,
        stage: CrmStage,
    ) -> Self {
        let name_s: SharedString = name.into();
        let domain_s: SharedString = domain.into();
        let owner_s: SharedString = owner.into();
        Self {
            id: id.into(),
            name: name_s.clone(),
            category: category.into(),
            domain: domain_s.clone(),
            owner: owner_s.clone(),
            value,
            stage,
            contact: "Taylor Brooks".into(),
            contact_email: format!("ops@{domain_s}").into(),
            contact_role: "VP Operations".into(),
            last_contact: "Today".into(),
            next_step: "Security review".into(),
            renewal_date: "Oct 15".into(),
            notes: vec![CrmActivityNote::new(
                owner_s,
                format!("Shared enterprise pricing deck with {name_s} buying group."),
                "2d ago",
            )],
        }
    }

    /// Set primary contact details for the `Contacts` tab and inspector.
    pub fn with_contact(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        role: impl Into<SharedString>,
    ) -> Self {
        self.contact = name.into();
        self.contact_email = email.into();
        self.contact_role = role.into();
        self
    }

    /// Set the last-contact label shown in `Company CRM`.
    pub fn with_last_contact(mut self, last_contact: impl Into<SharedString>) -> Self {
        self.last_contact = last_contact.into();
        self
    }

    /// Set the next scheduled deal action.
    pub fn with_next_step(mut self, next_step: impl Into<SharedString>) -> Self {
        self.next_step = next_step.into();
        self
    }

    /// Set the contract renewal date shown in `Renewals`.
    pub fn with_renewal_date(mut self, renewal_date: impl Into<SharedString>) -> Self {
        self.renewal_date = renewal_date.into();
        self
    }

    /// Override initial timeline notes.
    pub fn with_notes(mut self, notes: Vec<CrmActivityNote>) -> Self {
        self.notes = notes;
        self
    }
}

/// Events emitted by [`CrmScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrmEvent {
    Navigate(SharedString),
    ViewChanged(CrmView),
    DisplayModeChanged(CrmDisplayMode),
    CompanySelected(SharedString),
    StageChanged {
        company_id: SharedString,
        stage: CrmStage,
    },
    CellEdited {
        company_id: SharedString,
        column: SharedString,
        value: SharedString,
    },
    RowAdded(SharedString),
    AddRequested(SharedString),
    RowsDeleted(Vec<SharedString>),
    NoteAdded {
        company_id: SharedString,
        note: SharedString,
    },
    FavoriteToggled(bool),
}

impl EventEmitter<CrmEvent> for CrmScreen {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TagTone {
    Blue,
    Violet,
    Pink,
    Red,
    Green,
    Amber,
    Gray,
}

fn category_tone(category: &str) -> TagTone {
    match category.trim().to_ascii_lowercase().as_str() {
        "planning" | "crm" => TagTone::Blue,
        "payments" => TagTone::Violet,
        "design" => TagTone::Pink,
        "e-commerce" | "ecommerce" => TagTone::Amber,
        "security" => TagTone::Green,
        _ => TagTone::Gray,
    }
}

fn render_dtc_tag(label: impl Into<SharedString>, tone: TagTone, cx: &App) -> Div {
    let p = ProPalette::get(cx);
    let (bg, fg): (Hsla, Hsla) = match (p.dark, tone) {
        (false, TagTone::Blue) => (hsla(215. / 360., 1.0, 0.44, 0.08), rgb(0x0190fe).into()),
        (true, TagTone::Blue) => (hsla(218. / 360., 1.0, 0.65, 0.16), rgb(0x8bcbfe).into()),
        (false, TagTone::Violet) => (
            Hsla::from(rgb(0xa651f0)).opacity(0.09),
            rgb(0x8a35d6).into(),
        ),
        (true, TagTone::Violet) => (
            Hsla::from(rgb(0xb98af0)).opacity(0.18),
            rgb(0xb98af0).into(),
        ),
        (false, TagTone::Pink) => (
            Hsla::from(rgb(0xee3e9e)).opacity(0.08),
            rgb(0xc21f78).into(),
        ),
        (true, TagTone::Pink) => (
            Hsla::from(rgb(0xf06db5)).opacity(0.16),
            rgb(0xf06db5).into(),
        ),
        (false, TagTone::Red) => (
            Hsla::from(rgb(0xf04438)).opacity(0.09),
            rgb(0xd92d20).into(),
        ),
        (true, TagTone::Red) => (
            Hsla::from(rgb(0xff6b5f)).opacity(0.16),
            rgb(0xff8a7a).into(),
        ),
        (false, TagTone::Green) => (
            Hsla::from(rgb(0x5fc88a)).opacity(0.12),
            rgb(0x1e8a4f).into(),
        ),
        (true, TagTone::Green) => (
            Hsla::from(rgb(0x7dcf9f)).opacity(0.15),
            rgb(0x7dcf9f).into(),
        ),
        (false, TagTone::Amber) => (
            Hsla::from(rgb(0xe8b000)).opacity(0.12),
            rgb(0x8f6c00).into(),
        ),
        (true, TagTone::Amber) => (
            Hsla::from(rgb(0xe6c24d)).opacity(0.15),
            rgb(0xe6c24d).into(),
        ),
        (false, TagTone::Gray) => (hsla(0., 0., 0., 0.05), p.text),
        (true, TagTone::Gray) => (hsla(0., 0., 1., 0.08), p.text),
    };
    div()
        .h(px(20.))
        .px(px(6.))
        .rounded(px(5.))
        .bg(bg)
        .text_color(fg)
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .flex()
        .items_center()
        .whitespace_nowrap()
        .child(label.into())
}

fn render_company_logo(id: &str, name: &str, side: f32, cx: &App) -> AnyElement {
    let key = id.trim().to_ascii_lowercase();
    let glyph_side = (side * 0.66).round().max(10.);
    let inner: Option<AnyElement> = match key.as_str() {
        "notion" => Some(pro_svg(
            "0 0 24 24",
            MARK_NOTION,
            glyph_side,
            rgb(0x171717).into(),
        )),
        "linear" => Some(pro_svg(
            "0 0 24 24",
            MARK_LINEAR,
            glyph_side,
            rgb(0x5e6ad2).into(),
        )),
        "figma" => Some(pro_svg(
            "0 0 24 24",
            MARK_FIGMA,
            glyph_side,
            rgb(0xf24e1e).into(),
        )),
        "stripe" => Some(pro_svg(
            "0 0 24 24",
            MARK_STRIPE,
            glyph_side,
            rgb(0x635bff).into(),
        )),
        "shopify" => Some(pro_svg(
            "0 0 24 24",
            MARK_SHOPIFY,
            glyph_side,
            rgb(0x5fc88a).into(),
        )),
        "vercel" => Some(pro_svg(
            "0 0 24 24",
            MARK_VERCEL,
            glyph_side,
            rgb(0x171717).into(),
        )),
        "airtable" => Some(pro_svg(
            "0 0 24 24",
            MARK_AIRTABLE,
            glyph_side,
            rgb(0x18bfff).into(),
        )),
        "hubspot" => Some(pro_svg(
            "0 0 24 24",
            MARK_HUBSPOT,
            glyph_side,
            rgb(0xff7a59).into(),
        )),
        "clickup" => Some(pro_svg(
            "0 0 24 24",
            MARK_CLICKUP,
            glyph_side,
            rgb(0xa651f0).into(),
        )),
        "webflow" => Some(pro_svg(
            "0 0 24 24",
            MARK_WEBFLOW,
            glyph_side,
            rgb(0x0190fe).into(),
        )),
        _ => pro_app_artwork(name, glyph_side, cx),
    };

    let mark = inner.unwrap_or_else(|| {
        let initial: String = name
            .trim()
            .chars()
            .next()
            .unwrap_or('C')
            .to_uppercase()
            .collect();
        div()
            .text_size(px(10.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(0x525252))
            .child(initial)
            .into_any_element()
    });

    div()
        .size(px(side))
        .flex_none()
        .rounded(px(4.5))
        .bg(rgb(0xffffff))
        .border_1()
        .border_color(hsla(0., 0., 0., 0.08))
        .flex()
        .items_center()
        .justify_center()
        .child(mark)
        .into_any_element()
}

fn format_thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (digits.len() - idx).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn parse_currency_u64(raw: &str) -> Option<u64> {
    let cleaned: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if cleaned.is_empty() {
        None
    } else {
        cleaned.parse::<u64>().ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SortDir {
    Asc,
    Desc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PipelineMetrics {
    total_pipeline: u64,
    weighted_forecast: u64,
    closed_won: u64,
    active_count: usize,
}

fn pipeline_summary(companies: &[CrmCompany]) -> PipelineMetrics {
    let mut total_pipeline = 0u64;
    let mut weighted_forecast = 0u64;
    let mut closed_won = 0u64;
    let mut active_count = 0usize;
    for c in companies {
        total_pipeline = total_pipeline.saturating_add(c.value);
        weighted_forecast =
            weighted_forecast.saturating_add(c.value * u64::from(c.stage.probability()) / 100);
        if c.stage == CrmStage::Won {
            closed_won = closed_won.saturating_add(c.value);
        }
        if !matches!(c.stage, CrmStage::Won | CrmStage::Lost) {
            active_count += 1;
        }
    }
    PipelineMetrics {
        total_pipeline,
        weighted_forecast,
        closed_won,
        active_count,
    }
}

fn filter_and_sort_companies(
    companies: &[CrmCompany],
    stage_filter: CrmStageFilter,
    query: &str,
    sort: Option<(usize, SortDir)>,
) -> Vec<usize> {
    let q = query.trim().to_ascii_lowercase();
    let mut indices: Vec<usize> = companies
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            let stage_ok = match stage_filter {
                CrmStageFilter::All => true,
                CrmStageFilter::Stage(st) => c.stage == st,
            };
            let query_ok = q.is_empty()
                || c.name.to_ascii_lowercase().contains(&q)
                || c.category.to_ascii_lowercase().contains(&q)
                || c.domain.to_ascii_lowercase().contains(&q)
                || c.owner.to_ascii_lowercase().contains(&q)
                || c.contact.to_ascii_lowercase().contains(&q)
                || c.stage.as_str().to_ascii_lowercase().contains(&q);
            stage_ok && query_ok
        })
        .map(|(idx, _)| idx)
        .collect();

    if let Some((col, dir)) = sort {
        indices.sort_by(|&a, &b| {
            let ca = &companies[a];
            let cb = &companies[b];
            let ord = match col {
                0 => ca
                    .name
                    .to_ascii_lowercase()
                    .cmp(&cb.name.to_ascii_lowercase()),
                1 => ca
                    .category
                    .to_ascii_lowercase()
                    .cmp(&cb.category.to_ascii_lowercase()),
                2 => ca
                    .domain
                    .to_ascii_lowercase()
                    .cmp(&cb.domain.to_ascii_lowercase()),
                3 => ca
                    .owner
                    .to_ascii_lowercase()
                    .cmp(&cb.owner.to_ascii_lowercase()),
                4 => ca.value.cmp(&cb.value),
                5 => ca.stage.sort_rank().cmp(&cb.stage.sort_rank()),
                _ => ca.last_contact.cmp(&cb.last_contact),
            };
            match dir {
                SortDir::Asc => ord,
                SortDir::Desc => ord.reverse(),
            }
        });
    }
    indices
}

fn default_companies() -> Vec<CrmCompany> {
    vec![
        CrmCompany::new(
            "notion",
            "Notion",
            "Productivity",
            "notion.so",
            "Mara Klein",
            48_000,
            CrmStage::Negotiation,
        )
        .with_contact("Elena Rostova", "elena@notion.so", "VP Operations")
        .with_last_contact("Today")
        .with_next_step("Redline MSA clause 4")
        .with_renewal_date("Oct 14")
        .with_notes(vec![
            CrmActivityNote::new(
                "Mara Klein",
                "Legal reviewed redlines on SSO + audit log addendum; final sign-off Friday.",
                "Today · 10:15",
            ),
            CrmActivityNote::new(
                "Elena Rostova",
                "Confirmed 320 seats for workspace rollout across product and ops.",
                "Mar 4",
            ),
        ]),
        CrmCompany::new(
            "linear",
            "Linear",
            "Planning",
            "linear.app",
            "Jonas Weber",
            12_500,
            CrmStage::Proposal,
        )
        .with_contact("Lukas Lindqvist", "lukas@linear.app", "Head of Engineering")
        .with_last_contact("Yesterday")
        .with_next_step("Demo triage sync")
        .with_renewal_date("Nov 02"),
        CrmCompany::new(
            "figma",
            "Figma",
            "Design",
            "figma.com",
            "Mara Klein",
            86_000,
            CrmStage::Won,
        )
        .with_contact("Clara Chen", "clara@figma.com", "Design Systems Lead")
        .with_last_contact("Mar 6")
        .with_next_step("Kickoff workshop")
        .with_renewal_date("Mar 06"),
        CrmCompany::new(
            "stripe",
            "Stripe",
            "Payments",
            "stripe.com",
            "Lea Braun",
            124_000,
            CrmStage::Qualified,
        )
        .with_contact("David Park", "david@stripe.com", "Enterprise Partnerships")
        .with_last_contact("Mar 5")
        .with_next_step("Security questionnaire")
        .with_renewal_date("Dec 01"),
        CrmCompany::new(
            "shopify",
            "Shopify",
            "E-commerce",
            "shopify.com",
            "Jonas Weber",
            9_800,
            CrmStage::New,
        )
        .with_contact("Hannah Moreau", "hannah@shopify.com", "Merchant Platform")
        .with_last_contact("Mar 4")
        .with_next_step("Discovery call")
        .with_renewal_date("Jan 18"),
        CrmCompany::new(
            "vercel",
            "Vercel",
            "Developer Tools",
            "vercel.com",
            "Lea Braun",
            62_000,
            CrmStage::Proposal,
        )
        .with_contact("Mateo Silva", "mateo@vercel.com", "VP Infrastructure")
        .with_last_contact("Mar 3")
        .with_next_step("Volume tier review")
        .with_renewal_date("Sep 28"),
        CrmCompany::new(
            "airtable",
            "Airtable",
            "Productivity",
            "airtable.com",
            "Mara Klein",
            37_500,
            CrmStage::Qualified,
        )
        .with_contact("Rachel Kim", "rachel@airtable.com", "Product Ops")
        .with_last_contact("Mar 1")
        .with_next_step("Stakeholder demo")
        .with_renewal_date("Nov 19"),
        CrmCompany::new(
            "hubspot",
            "HubSpot",
            "CRM",
            "hubspot.com",
            "Tom Fischer",
            21_000,
            CrmStage::New,
        )
        .with_contact("Owen Gallagher", "owen@hubspot.com", "RevOps Director")
        .with_last_contact("Feb 27")
        .with_next_step("Intro sequence")
        .with_renewal_date("Feb 10"),
        CrmCompany::new(
            "clickup",
            "ClickUp",
            "Planning",
            "clickup.com",
            "Tom Fischer",
            6_400,
            CrmStage::Lost,
        )
        .with_contact("Sofia Patel", "sofia@clickup.com", "Workspace Admin")
        .with_last_contact("Feb 25")
        .with_next_step("Revisit Q4")
        .with_renewal_date("Aug 30"),
        CrmCompany::new(
            "webflow",
            "Webflow",
            "Design",
            "webflow.com",
            "Lea Braun",
            18_900,
            CrmStage::Negotiation,
        )
        .with_contact("Noah Bennett", "noah@webflow.com", "Brand Studio Lead")
        .with_last_contact("Feb 24")
        .with_next_step("Procurement approval")
        .with_renewal_date("Oct 29"),
    ]
}

const CATEGORIES: [&str; 7] = [
    "Productivity",
    "Planning",
    "Design",
    "Payments",
    "E-commerce",
    "Developer Tools",
    "CRM",
];

const OWNERS: [&str; 4] = ["Mara Klein", "Jonas Weber", "Lea Braun", "Tom Fischer"];

const SAVED_VIEWS: [&str; 4] = [
    "Head of Sales",
    "Enterprise EMEA",
    "Founder-led",
    "Partner channel",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum CellPickerKind {
    Category,
    Owner,
    Stage,
}

/// Interactive CRM workspace screen with editable table, pipeline views, and account inspector.
pub struct CrmScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    loading: bool,
    empty: bool,
    active_view: CrmView,
    display_mode: CrmDisplayMode,
    stage_filter: CrmStageFilter,
    toolbar_open: bool,
    composer_open: bool,
    saved_view: SharedString,
    saved_view_menu_open: bool,
    all_views_menu_open: bool,
    starred: bool,
    companies: Vec<CrmCompany>,
    undo_stack: Vec<Vec<CrmCompany>>,
    redo_stack: Vec<Vec<CrmCompany>>,
    selected_rows: Vec<SharedString>,
    active_cell: Option<(usize, usize)>,
    editing_cell: Option<(usize, usize)>,
    picker_cell: Option<(usize, usize, CellPickerKind)>,
    sort: Option<(usize, SortDir)>,
    inspector_id: Option<SharedString>,
    composer_stage: CrmStage,
    composer_owner_idx: usize,
    cell_editor: Option<Entity<Editor>>,
    search_editor: Option<Entity<Editor>>,
    new_name_editor: Option<Entity<Editor>>,
    new_domain_editor: Option<Entity<Editor>>,
    new_value_editor: Option<Entity<Editor>>,
    note_editor: Option<Entity<Editor>>,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
    anim_start: Instant,
}

impl CrmScreen {
    /// Construct a [`CrmScreen`] with the canonical `Acme` sales workspace and 10 sample accounts.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let groups = vec![
                ShellNavGroup::primary(vec![
                    ShellNavItem::new("home", tr(shell_cx, "Home", "首页"), ProIcon::Home),
                    ShellNavItem::new(
                        "companies",
                        tr(shell_cx, "Companies", "公司客户"),
                        ProIcon::Building,
                    )
                    .with_count(284),
                    ShellNavItem::new(
                        "contacts",
                        tr(shell_cx, "Contacts", "联系人"),
                        ProIcon::Users,
                    ),
                    ShellNavItem::new("deals", tr(shell_cx, "Deals", "商机"), ProIcon::Coins),
                    ShellNavItem::new(
                        "reports",
                        tr(shell_cx, "Reports", "报表"),
                        ProIcon::UsageChart,
                    ),
                ]),
                ShellNavGroup::named(
                    tr(shell_cx, "Pipelines", "商机管道"),
                    vec![
                        ShellNavItem::new(
                            "new-business",
                            tr(shell_cx, "New business", "新业务"),
                            ProIcon::Globe,
                        ),
                        ShellNavItem::new(
                            "renewals",
                            tr(shell_cx, "Renewals", "续约"),
                            ProIcon::Refresh,
                        ),
                        ShellNavItem::new(
                            "partnerships",
                            tr(shell_cx, "Partnerships", "合作方"),
                            ProIcon::Users,
                        ),
                    ],
                ),
            ];
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_workspace("Acme")
                .with_user("Alex Morgan", "alex@acme.ai")
                .with_side_cards(true)
                .with_groups(groups)
                .with_active(tr(shell_cx, "Companies", "公司客户"))
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
                let low = label.to_ascii_lowercase();
                if low.contains("deal") || low.contains("商机") || low.contains("new business") {
                    this.active_view = CrmView::Deals;
                } else if low.contains("contact") || low.contains("联系人") {
                    this.active_view = CrmView::Contacts;
                } else if low.contains("renewal") || low.contains("续约") {
                    this.active_view = CrmView::Renewals;
                } else if low.contains("compan") || low.contains("客户") {
                    this.active_view = CrmView::Companies;
                }
                cx.emit(CrmEvent::Navigate(label.clone()));
                cx.notify();
            }
            AppShellEvent::PanelOpen(false) => {
                this.inspector_id = None;
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
            active_view: CrmView::Companies,
            display_mode: CrmDisplayMode::Table,
            stage_filter: CrmStageFilter::All,
            toolbar_open: false,
            composer_open: false,
            saved_view: "Head of Sales".into(),
            saved_view_menu_open: false,
            all_views_menu_open: false,
            starred: false,
            companies: default_companies(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            selected_rows: Vec::new(),
            active_cell: None,
            editing_cell: None,
            picker_cell: None,
            sort: None,
            inspector_id: None,
            composer_stage: CrmStage::Qualified,
            composer_owner_idx: 0,
            cell_editor: None,
            search_editor: None,
            new_name_editor: None,
            new_domain_editor: None,
            new_value_editor: None,
            note_editor: None,
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

    /// Configure the initial empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Select the initially open CRM view tab.
    pub fn with_view(mut self, view: CrmView) -> Self {
        self.active_view = view;
        self
    }

    /// Configure table or pipeline board mode.
    pub fn with_display_mode(mut self, mode: CrmDisplayMode) -> Self {
        self.display_mode = mode;
        self
    }

    /// Replace the backing company records.
    pub fn with_companies(mut self, companies: Vec<CrmCompany>) -> Self {
        self.companies = companies;
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

    /// Return the currently selected CRM view tab.
    pub fn active_view(&self) -> CrmView {
        self.active_view
    }

    /// Return the active table or pipeline board display mode.
    pub fn display_mode(&self) -> CrmDisplayMode {
        self.display_mode
    }

    /// Return the current company records.
    pub fn companies(&self) -> &[CrmCompany] {
        &self.companies
    }

    /// Access the underlying [`AppShell`].
    pub fn shell(&self) -> &Entity<AppShell> {
        &self.shell
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.companies.clone());
        if self.undo_stack.len() > 32 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self, cx: &mut Context<Self>) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.companies.clone());
            self.companies = prev;
            self.editing_cell = None;
            self.picker_cell = None;
            cx.notify();
        }
    }

    fn redo(&mut self, cx: &mut Context<Self>) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.companies.clone());
            self.companies = next;
            self.editing_cell = None;
            self.picker_cell = None;
            cx.notify();
        }
    }

    fn open_inspector(&mut self, id: SharedString, cx: &mut Context<Self>) {
        self.inspector_id = Some(id.clone());
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(CrmEvent::CompanySelected(id));
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.cell_editor.is_none() {
            let ph: SharedString = tr(cx, "Value", "值").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Submitted | EditEvent::Blurred => {
                    this.commit_cell_edit(cx);
                }
                EditEvent::Escape => {
                    this.editing_cell = None;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.cell_editor = Some(ed);
        }

        if self.search_editor.is_none() {
            let ph: SharedString =
                tr(cx, "Filter companies or owners…", "筛选客户或负责人…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
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

        if self.new_name_editor.is_none() {
            let ph: SharedString = tr(cx, "Company name", "公司名称").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            self.new_name_editor = Some(ed);
        }

        if self.new_domain_editor.is_none() {
            let ph: SharedString = "acme.com".into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            self.new_domain_editor = Some(ed);
        }

        if self.new_value_editor.is_none() {
            let ph: SharedString = "25,000".into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            self.new_value_editor = Some(ed);
        }

        if self.note_editor.is_none() {
            let ph: SharedString = tr(cx, "Log a call or note…", "记录通话或跟进备注…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Submitted) {
                    this.submit_note_without_window(cx);
                }
            })
            .detach();
            self.note_editor = Some(ed);
        }
    }

    fn cell_text(&self, company_idx: usize, col: usize) -> String {
        let Some(c) = self.companies.get(company_idx) else {
            return String::new();
        };
        match (self.active_view, col) {
            (CrmView::Contacts, 0) => c.contact.to_string(),
            (CrmView::Contacts, 1) => c.name.to_string(),
            (CrmView::Contacts, 2) => c.contact_email.to_string(),
            (CrmView::Contacts, 3) => c.contact_role.to_string(),
            (CrmView::Contacts, 4) => c.owner.to_string(),
            (CrmView::Contacts, _) => c.last_contact.to_string(),
            (CrmView::Renewals, 0) => c.name.to_string(),
            (CrmView::Renewals, 1) => format_thousands(c.value),
            (CrmView::Renewals, 2) => c.renewal_date.to_string(),
            (CrmView::Renewals, 3) => c.stage.as_str().to_string(),
            (CrmView::Renewals, 4) => c.owner.to_string(),
            (CrmView::Renewals, _) => c.next_step.to_string(),
            (_, 0) => c.name.to_string(),
            (_, 1) => c.category.to_string(),
            (_, 2) => c.domain.to_string(),
            (_, 3) => c.owner.to_string(),
            (_, 4) => format_thousands(c.value),
            (_, 5) => c.stage.as_str().to_string(),
            (_, _) => c.last_contact.to_string(),
        }
    }

    fn cell_picker_kind(&self, col: usize) -> Option<CellPickerKind> {
        match (self.active_view, col) {
            (CrmView::Companies | CrmView::Deals, 1) => Some(CellPickerKind::Category),
            (CrmView::Companies | CrmView::Deals, 3) => Some(CellPickerKind::Owner),
            (CrmView::Companies | CrmView::Deals, 5) => Some(CellPickerKind::Stage),
            (CrmView::Contacts, 4) => Some(CellPickerKind::Owner),
            (CrmView::Renewals, 3) => Some(CellPickerKind::Stage),
            (CrmView::Renewals, 4) => Some(CellPickerKind::Owner),
            _ => None,
        }
    }

    fn start_cell_interaction(
        &mut self,
        company_idx: usize,
        col: usize,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_cell = Some((company_idx, col));
        if let Some(kind) = self.cell_picker_kind(col) {
            self.editing_cell = None;
            self.picker_cell = Some((company_idx, col, kind));
            cx.notify();
            return;
        }
        self.picker_cell = None;
        let current = self.cell_text(company_idx, col);
        if let Some(ed) = self.cell_editor.clone() {
            ed.update(cx, |e, cx| {
                e.set_value(current, w, cx);
                e.focus(w, cx);
            });
            self.editing_cell = Some((company_idx, col));
            cx.notify();
        }
    }

    fn commit_cell_edit(&mut self, cx: &mut Context<Self>) {
        let Some((company_idx, col)) = self.editing_cell.take() else {
            return;
        };
        let Some(ed) = &self.cell_editor else {
            return;
        };
        let next_val = ed.read(cx).value().trim().to_string();
        if company_idx >= self.companies.len() {
            return;
        }
        let prev_val = self.cell_text(company_idx, col);
        if next_val == prev_val {
            cx.notify();
            return;
        }
        self.push_undo();
        let company_id = self.companies[company_idx].id.clone();
        let col_name: &'static str = match (self.active_view, col) {
            (CrmView::Contacts, 0) => {
                self.companies[company_idx].contact = next_val.clone().into();
                "contact"
            }
            (CrmView::Contacts, 2) => {
                self.companies[company_idx].contact_email = next_val.clone().into();
                "email"
            }
            (CrmView::Contacts, 3) => {
                self.companies[company_idx].contact_role = next_val.clone().into();
                "role"
            }
            (CrmView::Contacts, _) => {
                self.companies[company_idx].last_contact = next_val.clone().into();
                "last_contact"
            }
            (CrmView::Renewals, 1) => {
                if let Some(v) = parse_currency_u64(&next_val) {
                    self.companies[company_idx].value = v;
                }
                "value"
            }
            (CrmView::Renewals, 2) => {
                self.companies[company_idx].renewal_date = next_val.clone().into();
                "renewal_date"
            }
            (CrmView::Renewals, _) => {
                self.companies[company_idx].next_step = next_val.clone().into();
                "next_step"
            }
            (_, 0) => {
                if !next_val.is_empty() {
                    self.companies[company_idx].name = next_val.clone().into();
                }
                "name"
            }
            (_, 2) => {
                self.companies[company_idx].domain = next_val.clone().into();
                "domain"
            }
            (_, 4) => {
                if let Some(v) = parse_currency_u64(&next_val) {
                    self.companies[company_idx].value = v;
                }
                "value"
            }
            (_, _) => {
                self.companies[company_idx].last_contact = next_val.clone().into();
                "last_contact"
            }
        };
        cx.emit(CrmEvent::CellEdited {
            company_id,
            column: col_name.into(),
            value: next_val.into(),
        });
        cx.notify();
    }

    fn add_quick_row(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.push_undo();
        self.empty = false;
        let n = self.companies.len() + 1;
        let id: SharedString = format!("company-{n}").into();
        let name: SharedString = format!("New Company {n}").into();
        let domain: SharedString = format!("company{n}.com").into();
        let row = CrmCompany::new(
            id.clone(),
            name,
            "Productivity",
            domain,
            "Mara Klein",
            15_000,
            CrmStage::New,
        )
        .with_last_contact("Just now");
        self.companies.push(row);
        let new_idx = self.companies.len() - 1;
        cx.emit(CrmEvent::RowAdded(id));
        self.start_cell_interaction(new_idx, 0, w, cx);
    }

    fn submit_composer(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let raw_name = self
            .new_name_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        if raw_name.is_empty() {
            return;
        }
        let raw_domain = self
            .new_domain_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        let raw_val = self
            .new_value_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();

        let slug: String = raw_name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        let domain = if raw_domain.is_empty() {
            format!("{}.com", if slug.is_empty() { "account" } else { &slug })
        } else {
            raw_domain
        };
        let value = parse_currency_u64(&raw_val).unwrap_or(24_000);
        let owner = OWNERS[self.composer_owner_idx % OWNERS.len()];
        let id: SharedString = format!(
            "{}-{}",
            if slug.is_empty() { "deal" } else { &slug },
            self.companies.len() + 1
        )
        .into();

        self.push_undo();
        self.empty = false;
        self.companies.insert(
            0,
            CrmCompany::new(
                id.clone(),
                raw_name,
                "Productivity",
                domain,
                owner,
                value,
                self.composer_stage,
            )
            .with_last_contact("Just now"),
        );
        if let Some(ed) = &self.new_name_editor {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        if let Some(ed) = &self.new_domain_editor {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        if let Some(ed) = &self.new_value_editor {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        self.composer_open = false;
        cx.emit(CrmEvent::RowAdded(id));
        cx.notify();
    }

    fn submit_note_without_window(&mut self, cx: &mut Context<Self>) {
        let Some(target_id) = self.inspector_id.clone() else {
            return;
        };
        let Some(ed) = &self.note_editor else {
            return;
        };
        let text = ed.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        if let Some(c) = self.companies.iter_mut().find(|c| c.id == target_id) {
            c.notes.insert(
                0,
                CrmActivityNote::new("Alex Morgan", text.clone(), "Just now"),
            );
            c.last_contact = "Just now".into();
            cx.emit(CrmEvent::NoteAdded {
                company_id: target_id,
                note: text.into(),
            });
            cx.notify();
        }
    }

    fn delete_selected_rows(&mut self, cx: &mut Context<Self>) {
        if self.selected_rows.is_empty() {
            return;
        }
        self.push_undo();
        let removed = std::mem::take(&mut self.selected_rows);
        self.companies.retain(|c| !removed.contains(&c.id));
        self.active_cell = None;
        self.editing_cell = None;
        self.picker_cell = None;
        cx.emit(CrmEvent::RowsDeleted(removed));
        cx.notify();
    }

    fn on_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        let cmd = ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control;
        if cmd && key.eq_ignore_ascii_case("z") {
            if ev.keystroke.modifiers.shift {
                self.redo(cx);
            } else {
                self.undo(cx);
            }
            return;
        }
        if self.editing_cell.is_some() {
            return;
        }
        if key == "escape" {
            self.picker_cell = None;
            self.saved_view_menu_open = false;
            self.all_views_menu_open = false;
            self.composer_open = false;
            cx.notify();
            return;
        }
        if key == "/" {
            self.toolbar_open = true;
            if let Some(ed) = self.search_editor.clone() {
                ed.update(cx, |e, cx| e.focus(w, cx));
            }
            cx.notify();
            return;
        }

        let query = self
            .search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        let visible =
            filter_and_sort_companies(&self.companies, self.stage_filter, &query, self.sort);
        if visible.is_empty() {
            return;
        }

        let col_count = match self.active_view {
            CrmView::Contacts | CrmView::Renewals => 6usize,
            _ => 7usize,
        };

        match key {
            "down" | "j" => {
                let (cur_idx, col) = self.active_cell.unwrap_or((visible[0], 0));
                let pos = visible.iter().position(|&i| i == cur_idx).unwrap_or(0);
                let next = visible[(pos + 1).min(visible.len() - 1)];
                self.active_cell = Some((next, col));
                cx.notify();
            }
            "up" | "k" => {
                let (cur_idx, col) = self.active_cell.unwrap_or((visible[0], 0));
                let pos = visible.iter().position(|&i| i == cur_idx).unwrap_or(0);
                let next = visible[pos.saturating_sub(1)];
                self.active_cell = Some((next, col));
                cx.notify();
            }
            "right" => {
                let (cur_idx, col) = self.active_cell.unwrap_or((visible[0], 0));
                self.active_cell = Some((cur_idx, (col + 1).min(col_count - 1)));
                cx.notify();
            }
            "left" => {
                let (cur_idx, col) = self.active_cell.unwrap_or((visible[0], 0));
                self.active_cell = Some((cur_idx, col.saturating_sub(1)));
                cx.notify();
            }
            "enter" => {
                if let Some((r, c)) = self.active_cell {
                    self.start_cell_interaction(r, c, w, cx);
                }
            }
            "i" => {
                let idx = self.active_cell.map(|(r, _)| r).unwrap_or(visible[0]);
                if let Some(comp) = self.companies.get(idx) {
                    let cid = comp.id.clone();
                    if self.inspector_id.as_ref() == Some(&cid) {
                        self.inspector_id = None;
                        self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
                        cx.notify();
                    } else {
                        self.open_inspector(cid, cx);
                    }
                }
            }
            _ => {}
        }
    }

    fn render_checkbox<F>(
        &self,
        id: ElementId,
        aria: SharedString,
        checked: bool,
        mixed: bool,
        cx: &Context<Self>,
        on_press: F,
    ) -> Stateful<Div>
    where
        F: Fn(&mut Self, &mut Window, &mut Context<Self>) + Clone + 'static,
    {
        let p = ProPalette::get(cx);
        let active = checked || mixed;
        let bg = if active { p.primary } else { p.card };
        let ring = if p.dark {
            hsla(0., 0., 1., 0.26)
        } else {
            hsla(0., 0., 0., 0.22)
        };
        let el = div()
            .id(id)
            .role(Role::CheckBox)
            .aria_label(aria)
            .tab_index(0)
            .size(px(14.))
            .rounded(px(4.))
            .bg(bg)
            .when(!active, |d| d.border_1().border_color(ring))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .when(active, |d| {
                d.child(pro_icon(
                    if mixed && !checked {
                        ProIcon::Minus
                    } else {
                        ProIcon::Check
                    },
                    10.,
                    p.on_primary,
                ))
            });
        activate(el, cx, on_press)
    }

    fn render_tab_badge(view: CrmView, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        let (bg, mark) = match view {
            CrmView::Companies => (rgb(0xff7a59), MARK_HUBSPOT),
            CrmView::Deals => (rgb(0x635bff), MARK_STRIPE),
            CrmView::Contacts => (rgb(0x18bfff), MARK_AIRTABLE),
            CrmView::Renewals => (rgb(0x171717), MARK_NOTION),
        };
        div()
            .size(px(18.))
            .flex_none()
            .rounded(px(5.))
            .bg(bg)
            .when(view == CrmView::Renewals && p.dark, |d| {
                d.border_1().border_color(p.line_strong)
            })
            .flex()
            .items_center()
            .justify_center()
            .child(pro_svg("0 0 24 24", mark, 11., rgb(0xffffff).into()))
    }

    fn render_kpi_and_toolbar(&mut self, compact: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let metrics = pipeline_summary(&self.companies);
        let kpis = [
            (
                tr(cx, "Total pipeline", "总漏斗金额"),
                format!("${}", format_thousands(metrics.total_pipeline)),
                ProTone::Blue,
            ),
            (
                tr(cx, "Weighted forecast", "加权预测"),
                format!("${}", format_thousands(metrics.weighted_forecast)),
                ProTone::Purple,
            ),
            (
                tr(cx, "Closed won", "已赢单"),
                format!("${}", format_thousands(metrics.closed_won)),
                ProTone::Green,
            ),
            (
                tr(cx, "Open deals", "活跃商机"),
                format!("{} {}", metrics.active_count, tr(cx, "active", "进行中")),
                ProTone::Neutral,
            ),
        ];

        let mut kpi_row = div().px(px(12.)).pb(px(10.)).flex().flex_wrap().gap(px(8.));
        for (lbl, val, tone) in kpis {
            kpi_row = kpi_row.child(
                div()
                    .flex_1()
                    .min_w(px(if compact { 132. } else { 150. }))
                    .p(px(10.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_size(px(11.5)).text_color(p.muted).child(lbl))
                            .child(pro_badge(
                                match tone {
                                    ProTone::Green => "+14%",
                                    ProTone::Blue => "Q1",
                                    ProTone::Purple => "Prob.",
                                    _ => "Live",
                                },
                                tone,
                                false,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.strong)
                            .child(val),
                    ),
            );
        }

        let all_count = self.companies.len();
        let stage_sensor = pro_selection_sensor();
        let stage_key: SharedString = match self.stage_filter {
            CrmStageFilter::All => "all".into(),
            CrmStageFilter::Stage(s) => s.as_str().into(),
        };
        let mut stage_pills = div()
            .relative()
            .flex()
            .items_center()
            .flex_wrap()
            .gap(px(4.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "stage-flt-pill"),
                stage_key,
                stage_sensor.clone(),
                6.,
                p.select,
                Some(p.line_strong),
                false,
            ));
        let all_active = self.stage_filter == CrmStageFilter::All;
        stage_pills = stage_pills.child(activate(
            div()
                .id(pro_eid(&self.id, "flt-all"))
                .role(Role::Tab)
                .tab_index(0)
                .relative()
                .h(px(26.))
                .px(px(9.))
                .rounded(px(6.))
                .border_1()
                .border_color(if all_active {
                    transparent_black()
                } else {
                    p.line
                })
                .when(all_active, |d| d.child(pro_mark_selected(&stage_sensor)))
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.))
                .font_weight(if all_active {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if all_active { p.ink } else { p.text })
                .cursor_pointer()
                .when(!all_active, |d| d.hover(move |s| s.bg(p.hover)))
                .child(tr(cx, "All", "全部"))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(p.faint)
                        .child(all_count.to_string()),
                ),
            cx,
            |this, _, cx| {
                this.stage_filter = CrmStageFilter::All;
                cx.notify();
            },
        ));

        for st in [
            CrmStage::Qualified,
            CrmStage::Proposal,
            CrmStage::Negotiation,
            CrmStage::Won,
            CrmStage::New,
            CrmStage::Lost,
        ] {
            let cnt = self.companies.iter().filter(|c| c.stage == st).count();
            let on = self.stage_filter == CrmStageFilter::Stage(st);
            stage_pills = stage_pills.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("flt-{}", st.as_str())))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .h(px(26.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if on { transparent_black() } else { p.line })
                    .when(on, |d| d.child(pro_mark_selected(&stage_sensor)))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.))
                    .font_weight(if on {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if on { p.ink } else { p.text })
                    .cursor_pointer()
                    .when(!on, |d| d.hover(move |s| s.bg(p.hover)))
                    .child(st.label(cx))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.faint)
                            .child(cnt.to_string()),
                    ),
                cx,
                move |this, _, cx| {
                    this.stage_filter = CrmStageFilter::Stage(st);
                    cx.notify();
                },
            ));
        }

        let has_query = self
            .search_editor
            .as_ref()
            .is_some_and(|e| !e.read(cx).value().is_empty());
        let search_ed = self.search_editor.clone();
        let search_box = div()
            .h(px(28.))
            .w(px(if compact { 150. } else { 188. }))
            .px(px(8.))
            .rounded(px(7.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(6.))
            .child(pro_icon(ProIcon::Search, 13., p.muted))
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

        let is_table = self.display_mode == CrmDisplayMode::Table;
        let mode_sensor = pro_selection_sensor();
        let mode_switch = div()
            .relative()
            .h(px(28.))
            .p(px(2.))
            .rounded(px(7.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "mode-pill"),
                if is_table { "table" } else { "board" },
                mode_sensor.clone(),
                5.,
                p.card,
                Some(p.line),
                true,
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "mode-table"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Table view", "表格视图"))
                    .tab_index(0)
                    .relative()
                    .size(px(24.))
                    .rounded(px(5.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .when(is_table, |d| d.child(pro_mark_selected(&mode_sensor)))
                    .when(!is_table, |d| d.hover(move |s| s.bg(p.hover)))
                    .child(pro_icon(
                        ProIcon::List,
                        14.,
                        if is_table { p.ink } else { p.muted },
                    )),
                cx,
                |this, _, cx| {
                    this.display_mode = CrmDisplayMode::Table;
                    cx.emit(CrmEvent::DisplayModeChanged(CrmDisplayMode::Table));
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "mode-board"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Pipeline board", "看板视图"))
                    .tab_index(0)
                    .relative()
                    .size(px(24.))
                    .rounded(px(5.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .when(!is_table, |d| d.child(pro_mark_selected(&mode_sensor)))
                    .when(is_table, |d| d.hover(move |s| s.bg(p.hover)))
                    .child(pro_icon(
                        ProIcon::Grid,
                        14.,
                        if !is_table { p.ink } else { p.muted },
                    )),
                cx,
                |this, _, cx| {
                    this.display_mode = CrmDisplayMode::Board;
                    cx.emit(CrmEvent::DisplayModeChanged(CrmDisplayMode::Board));
                    cx.notify();
                },
            ));

        let add_lbl = if self.active_view == CrmView::Deals {
            tr(cx, "New deal", "新建商机")
        } else {
            tr(cx, "Add company", "添加公司")
        };

        let show_deals_kpi = self.active_view == CrmView::Deals;
        div()
            .flex()
            .flex_col()
            .child(pro_fold(
                pro_eid(&self.id, "deals-kpi-fold"),
                show_deals_kpi,
                68.,
                0.,
                move || kpi_row.into_any_element(),
            ))
            .child(
                div()
                    .px(px(12.))
                    .pb(px(10.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(stage_pills)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(search_box)
                            .child(mode_switch)
                            .child(pro_btn(
                                pro_eid(&self.id, "open-composer"),
                                add_lbl,
                                Some(ProIcon::Plus),
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                |this, w, cx| {
                                    this.composer_open = !this.composer_open;
                                    if this.composer_open
                                        && let Some(ed) = this.new_name_editor.clone()
                                    {
                                        ed.update(cx, |e, cx| e.focus(w, cx));
                                    }
                                    cx.notify();
                                },
                            )),
                    ),
            )
    }

    fn render_composer(&self, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let owner_name = OWNERS[self.composer_owner_idx % OWNERS.len()];
        let stage = self.composer_stage;
        div()
            .mx(px(12.))
            .mb(px(10.))
            .p(px(10.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .h(px(28.))
                    .w(px(156.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .when_some(self.new_name_editor.clone(), |d, ed| {
                        d.child(div().flex_1().min_w_0().child(ed))
                    }),
            )
            .child(
                div()
                    .h(px(28.))
                    .w(px(126.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .when_some(self.new_domain_editor.clone(), |d, ed| {
                        d.child(div().flex_1().min_w_0().child(ed))
                    }),
            )
            .child(
                div()
                    .h(px(28.))
                    .w(px(96.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(div().text_size(px(12.)).text_color(p.muted).child("$"))
                    .when_some(self.new_value_editor.clone(), |d, ed| {
                        d.child(div().flex_1().min_w_0().child(ed))
                    }),
            )
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "cmp-stage"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(render_dtc_tag(stage.label(cx), stage.tag_tone(), cx))
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
                cx,
                |this, _, cx| {
                    let pos = CrmStage::ALL
                        .iter()
                        .position(|&s| s == this.composer_stage)
                        .unwrap_or(0);
                    this.composer_stage = CrmStage::ALL[(pos + 1) % CrmStage::ALL.len()];
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "cmp-owner"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.5))
                    .text_color(p.ink)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(pro_person_avatar(owner_name, 16., false, None, cx))
                    .child(owner_name)
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
                cx,
                |this, _, cx| {
                    this.composer_owner_idx = (this.composer_owner_idx + 1) % OWNERS.len();
                    cx.notify();
                },
            ))
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_btn(
                        pro_eid(&self.id, "cmp-cancel"),
                        tr(cx, "Cancel", "取消"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, _, cx| {
                            this.composer_open = false;
                            cx.notify();
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "cmp-save"),
                        tr(cx, "Save", "保存"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, w, cx| this.submit_composer(w, cx),
                    )),
            )
    }

    fn column_specs(&self, cx: &App) -> Vec<(&'static str, f32, AnyElement)> {
        let p = ProPalette::get(cx);
        let ic = p.muted;
        match self.active_view {
            CrmView::Contacts => vec![
                (
                    tr(cx, "Contact", "联系人"),
                    0.20,
                    pro_icon(ProIcon::User, 14., ic),
                ),
                (
                    tr(cx, "Company", "公司"),
                    0.18,
                    pro_svg("0 0 24 24", ICON_COL_TEXT, 14., ic),
                ),
                (
                    tr(cx, "Email", "邮箱"),
                    0.22,
                    pro_icon(ProIcon::Mail, 14., ic),
                ),
                (
                    tr(cx, "Role", "职位"),
                    0.16,
                    pro_svg("0 0 24 24", ICON_COL_TAG, 14., ic),
                ),
                (
                    tr(cx, "Owner", "负责人"),
                    0.13,
                    pro_icon(ProIcon::Users, 14., ic),
                ),
                (
                    tr(cx, "Last contact", "最近联系"),
                    0.11,
                    pro_icon(ProIcon::Calendar, 14., ic),
                ),
            ],
            CrmView::Renewals => vec![
                (
                    tr(cx, "Company", "公司"),
                    0.21,
                    pro_svg("0 0 24 24", ICON_COL_TEXT, 14., ic),
                ),
                (
                    tr(cx, "ARR", "年度合同额"),
                    0.14,
                    pro_icon(ProIcon::Coins, 14., ic),
                ),
                (
                    tr(cx, "Renewal date", "续约日期"),
                    0.15,
                    pro_icon(ProIcon::Calendar, 14., ic),
                ),
                (
                    tr(cx, "Stage", "阶段"),
                    0.16,
                    pro_svg("0 0 24 24", ICON_COL_STAGE, 14., ic),
                ),
                (
                    tr(cx, "Owner", "负责人"),
                    0.16,
                    pro_icon(ProIcon::User, 14., ic),
                ),
                (
                    tr(cx, "Next step", "下一步"),
                    0.18,
                    pro_icon(ProIcon::Activity, 14., ic),
                ),
            ],
            CrmView::Companies | CrmView::Deals => vec![
                (
                    tr(cx, "Company", "公司"),
                    0.18,
                    pro_svg("0 0 24 24", ICON_COL_TEXT, 14., ic),
                ),
                (
                    tr(cx, "Category", "行业分类"),
                    0.16,
                    pro_svg("0 0 24 24", ICON_COL_TAG, 14., ic),
                ),
                (
                    tr(cx, "Domain", "域名"),
                    0.15,
                    pro_icon(ProIcon::Link, 14., ic),
                ),
                (
                    tr(cx, "Owner", "负责人"),
                    0.15,
                    pro_icon(ProIcon::User, 14., ic),
                ),
                (
                    tr(cx, "Value", "金额"),
                    0.11,
                    pro_icon(ProIcon::Coins, 14., ic),
                ),
                (
                    tr(cx, "Stage", "阶段"),
                    0.13,
                    pro_svg("0 0 24 24", ICON_COL_STAGE, 14., ic),
                ),
                (
                    tr(cx, "Last contact", "最近联系"),
                    0.12,
                    pro_icon(ProIcon::Calendar, 14., ic),
                ),
            ],
        }
    }

    fn render_table_header(&self, visible_ids: &[SharedString], cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let all_checked =
            !visible_ids.is_empty() && visible_ids.iter().all(|id| self.selected_rows.contains(id));
        let some_checked = visible_ids.iter().any(|id| self.selected_rows.contains(id));
        let toggle_ids = visible_ids.to_vec();

        let head_row = div()
            .h(px(36.))
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .child(
                div()
                    .w(px(38.))
                    .h_full()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.render_checkbox(
                        pro_eid(&self.id, "chk-all"),
                        tr(cx, "Select all rows", "全选所有行").into(),
                        all_checked,
                        some_checked && !all_checked,
                        cx,
                        move |this, _, cx| {
                            if all_checked {
                                this.selected_rows.retain(|id| !toggle_ids.contains(id));
                            } else {
                                for id in &toggle_ids {
                                    if !this.selected_rows.contains(id) {
                                        this.selected_rows.push(id.clone());
                                    }
                                }
                            }
                            cx.notify();
                        },
                    )),
            );

        let mut cols_wrap = div().flex_1().min_w_0().h_full().flex().items_center();
        for (col_idx, (label, weight, icon_el)) in self.column_specs(cx).into_iter().enumerate() {
            let active_dir = self
                .sort
                .and_then(|(sc, dir)| (sc == col_idx).then_some(dir));
            cols_wrap = cols_wrap.child(
                div()
                    .w(relative(weight))
                    .min_w_0()
                    .h_full()
                    .px(px(6.))
                    .flex()
                    .items_center()
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("th-{col_idx}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .w_full()
                            .h(px(28.))
                            .px(px(6.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if active_dir.is_some() { p.ink } else { p.text })
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover).text_color(p.ink))
                            .child(icon_el)
                            .child(div().truncate().child(label))
                            .when_some(active_dir, |d, dir| {
                                d.child(div().ml_auto().child(pro_icon(
                                    match dir {
                                        SortDir::Asc => ProIcon::ChevronUp,
                                        SortDir::Desc => ProIcon::ChevronDown,
                                    },
                                    12.,
                                    p.blue,
                                )))
                            }),
                        cx,
                        move |this, _, cx| {
                            this.sort = match this.sort {
                                Some((c, SortDir::Asc)) if c == col_idx => {
                                    Some((col_idx, SortDir::Desc))
                                }
                                Some((c, SortDir::Desc)) if c == col_idx => None,
                                _ => Some((col_idx, SortDir::Asc)),
                            };
                            cx.notify();
                        },
                    )),
            );
        }

        head_row.child(cols_wrap)
    }

    fn render_skeleton_table(&self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let phase = if Theme::global(cx).reduced_motion {
            0.25
        } else {
            w.request_animation_frame();
            (self.anim_start.elapsed().as_secs_f32() * 0.75).fract()
        };

        let mut wrap = div()
            .w_full()
            .min_w(px(660.))
            .flex()
            .flex_col()
            .child(self.render_table_header(&[], cx));

        let widths: [[f32; 7]; 8] = [
            [72., 68., 62., 64., 46., 66., 42.],
            [64., 54., 66., 74., 42., 56., 58.],
            [60., 46., 60., 64., 48., 36., 38.],
            [58., 58., 64., 56., 54., 56., 38.],
            [66., 72., 70., 74., 38., 34., 38.],
            [56., 84., 62., 56., 46., 56., 38.],
            [68., 68., 72., 64., 46., 56., 38.],
            [68., 36., 74., 68., 44., 34., 44.],
        ];

        for (ri, bar_ws) in widths.into_iter().enumerate() {
            let ph = (phase + ri as f32 * 0.06).fract();
            let specs = self.column_specs(cx);
            let row_el = div()
                .h(px(40.))
                .w_full()
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .child(
                    div()
                        .w(px(38.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_skeleton_bar(14., 14., 4., ph, cx)),
                );

            let mut cols_el = div().flex_1().min_w_0().h_full().flex().items_center();
            for (ci, (_, weight, _)) in specs.into_iter().enumerate() {
                let bw = bar_ws.get(ci).copied().unwrap_or(52.);
                cols_el = cols_el.child(
                    div()
                        .w(relative(weight))
                        .min_w_0()
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .when(ci == 0, |d| {
                            d.child(pro_skeleton_bar(16., 16., 4.5, ph, cx))
                        })
                        .child(pro_skeleton_bar(
                            bw,
                            if ci == 1 || ci == 5 { 18. } else { 11. },
                            5.,
                            ph,
                            cx,
                        )),
                );
            }
            wrap = wrap.child(row_el.child(cols_el));
        }

        wrap.child(
            div()
                .h(px(44.))
                .w_full()
                .px(px(16.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_between()
                .child(pro_skeleton_bar(68., 12., 5., phase, cx))
                .child(pro_skeleton_bar(124., 10., 5., phase, cx)),
        )
    }

    fn render_cell_content(
        &self,
        company_idx: usize,
        col: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let c = &self.companies[company_idx];
        let underline_c = if p.dark {
            hsla(0., 0., 1., 0.22)
        } else {
            hsla(0., 0., 0., 0.16)
        };

        match (self.active_view, col) {
            (CrmView::Contacts, 0) => div()
                .flex()
                .items_center()
                .gap(px(8.))
                .min_w_0()
                .child(pro_person_avatar(&c.contact, 18., false, None, cx))
                .child(
                    div()
                        .truncate()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(c.contact.clone()),
                )
                .into_any_element(),
            (CrmView::Contacts, 1) => div()
                .flex()
                .items_center()
                .gap(px(8.))
                .min_w_0()
                .child(render_company_logo(&c.id, &c.name, 16., cx))
                .child(div().truncate().text_color(p.ink).child(c.name.clone()))
                .into_any_element(),
            (CrmView::Contacts, 2) => div()
                .border_b_1()
                .border_color(underline_c)
                .text_color(p.text)
                .truncate()
                .child(c.contact_email.clone())
                .into_any_element(),
            (CrmView::Contacts, 3) => div()
                .truncate()
                .text_color(p.text)
                .child(c.contact_role.clone())
                .into_any_element(),
            (CrmView::Contacts, 4) => div()
                .truncate()
                .text_color(p.text)
                .child(c.owner.clone())
                .into_any_element(),
            (CrmView::Contacts, _) => div()
                .truncate()
                .text_color(p.text)
                .child(c.last_contact.clone())
                .into_any_element(),
            (CrmView::Renewals, 0) => div()
                .flex()
                .items_center()
                .gap(px(8.))
                .min_w_0()
                .child(render_company_logo(&c.id, &c.name, 16., cx))
                .child(
                    div()
                        .truncate()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(c.name.clone()),
                )
                .into_any_element(),
            (CrmView::Renewals, 1) => div()
                .truncate()
                .text_color(p.text)
                .child(format_thousands(c.value))
                .into_any_element(),
            (CrmView::Renewals, 2) => div()
                .truncate()
                .text_color(p.text)
                .child(c.renewal_date.clone())
                .into_any_element(),
            (CrmView::Renewals, 3) => {
                render_dtc_tag(c.stage.label(cx), c.stage.tag_tone(), cx).into_any_element()
            }
            (CrmView::Renewals, 4) => div()
                .flex()
                .items_center()
                .gap(px(6.))
                .min_w_0()
                .child(pro_person_avatar(&c.owner, 16., false, None, cx))
                .child(div().truncate().text_color(p.text).child(c.owner.clone()))
                .into_any_element(),
            (CrmView::Renewals, _) => div()
                .truncate()
                .text_color(p.text)
                .child(c.next_step.clone())
                .into_any_element(),
            (_, 0) => {
                let cid = c.id.clone();
                let is_inspected = self.inspector_id.as_ref() == Some(&cid);
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_w_0()
                    .child(render_company_logo(&c.id, &c.name, 16., cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(c.name.clone()),
                    )
                    .when(
                        is_inspected || self.active_cell.is_some_and(|(r, _)| r == company_idx),
                        |d| {
                            d.child(
                                div()
                                    .id(pro_eid(&self.id, format!("insp-{}", c.id)))
                                    .role(Role::Button)
                                    .aria_label(tr(cx, "Inspect company", "查看客户详情"))
                                    .size(px(20.))
                                    .rounded(px(5.))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(p.chip_hover))
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                        cx.stop_propagation();
                                    })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.open_inspector(cid.clone(), cx);
                                    }))
                                    .child(pro_icon(ProIcon::SidebarRight, 12., p.muted)),
                            )
                        },
                    )
                    .into_any_element()
            }
            (_, 1) => render_dtc_tag(c.category.clone(), category_tone(&c.category), cx)
                .into_any_element(),
            (_, 2) => div()
                .border_b_1()
                .border_color(underline_c)
                .text_color(p.text)
                .truncate()
                .child(c.domain.clone())
                .into_any_element(),
            (_, 3) => div()
                .truncate()
                .text_color(p.text)
                .child(c.owner.clone())
                .into_any_element(),
            (_, 4) => div()
                .truncate()
                .text_color(p.text)
                .child(format_thousands(c.value))
                .into_any_element(),
            (_, 5) => render_dtc_tag(c.stage.label(cx), c.stage.tag_tone(), cx).into_any_element(),
            (_, _) => div()
                .truncate()
                .text_color(p.text)
                .child(c.last_contact.clone())
                .into_any_element(),
        }
    }

    fn render_picker_bar(
        &self,
        company_idx: usize,
        kind: CellPickerKind,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let Some(comp) = self.companies.get(company_idx) else {
            return div();
        };
        let company_id = comp.id.clone();

        let mut row = div()
            .px(px(46.))
            .py(px(6.))
            .bg(p.subtle)
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.));

        match kind {
            CellPickerKind::Stage => {
                let current = comp.stage;
                for st in CrmStage::ALL {
                    let selected = st == current;
                    let cid = company_id.clone();
                    row = row.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("pick-st-{}", st.as_str())))
                            .role(Role::ListBoxOption)
                            .tab_index(0)
                            .h(px(26.))
                            .px(px(6.))
                            .rounded(px(6.))
                            .when(selected, |d| d.bg(p.select))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .cursor_pointer()
                            .child(render_dtc_tag(st.label(cx), st.tag_tone(), cx))
                            .when(selected, |d| d.child(pro_icon(ProIcon::Check, 12., p.blue))),
                        cx,
                        move |this, _, cx| {
                            if company_idx < this.companies.len() {
                                this.push_undo();
                                this.companies[company_idx].stage = st;
                                this.picker_cell = None;
                                cx.emit(CrmEvent::StageChanged {
                                    company_id: cid.clone(),
                                    stage: st,
                                });
                                cx.notify();
                            }
                        },
                    ));
                }
            }
            CellPickerKind::Category => {
                let current = comp.category.clone();
                for cat in CATEGORIES {
                    let selected = current.as_ref() == cat;
                    let cid = company_id.clone();
                    row = row.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("pick-cat-{cat}")))
                            .role(Role::ListBoxOption)
                            .tab_index(0)
                            .h(px(26.))
                            .px(px(6.))
                            .rounded(px(6.))
                            .when(selected, |d| d.bg(p.select))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .cursor_pointer()
                            .child(render_dtc_tag(cat, category_tone(cat), cx))
                            .when(selected, |d| d.child(pro_icon(ProIcon::Check, 12., p.blue))),
                        cx,
                        move |this, _, cx| {
                            if company_idx < this.companies.len() {
                                this.push_undo();
                                this.companies[company_idx].category = cat.into();
                                this.picker_cell = None;
                                cx.emit(CrmEvent::CellEdited {
                                    company_id: cid.clone(),
                                    column: "category".into(),
                                    value: cat.into(),
                                });
                                cx.notify();
                            }
                        },
                    ));
                }
            }
            CellPickerKind::Owner => {
                let current = comp.owner.clone();
                for owner in OWNERS {
                    let selected = current.as_ref() == owner;
                    let cid = company_id.clone();
                    row = row.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("pick-own-{owner}")))
                            .role(Role::ListBoxOption)
                            .tab_index(0)
                            .h(px(26.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .when(selected, |d| d.bg(p.select))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.5))
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(pro_person_avatar(owner, 16., false, None, cx))
                            .child(owner)
                            .when(selected, |d| d.child(pro_icon(ProIcon::Check, 12., p.blue))),
                        cx,
                        move |this, _, cx| {
                            if company_idx < this.companies.len() {
                                this.push_undo();
                                this.companies[company_idx].owner = owner.into();
                                this.picker_cell = None;
                                cx.emit(CrmEvent::CellEdited {
                                    company_id: cid.clone(),
                                    column: "owner".into(),
                                    value: owner.into(),
                                });
                                cx.notify();
                            }
                        },
                    ));
                }
            }
        }

        row.child(div().ml_auto().child(pro_icon_btn(
            pro_eid(&self.id, "close-picker"),
            tr(cx, "Close picker", "关闭选择器"),
            ProIcon::Close,
            22.,
            false,
            cx,
            |this, _, cx| {
                this.picker_cell = None;
                cx.notify();
            },
        )))
    }

    fn render_data_table(&self, visible: &[usize], cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let visible_ids: Vec<SharedString> = visible
            .iter()
            .map(|&idx| self.companies[idx].id.clone())
            .collect();

        let mut table = div()
            .w_full()
            .min_w(px(660.))
            .flex()
            .flex_col()
            .child(self.render_table_header(&visible_ids, cx));

        let selected_tint = if p.dark {
            hsla(218. / 360., 1.0, 0.65, 0.14)
        } else {
            hsla(215. / 360., 1.0, 0.44, 0.05)
        };
        let row_hover = if p.dark {
            hsla(0., 0., 1., 0.025)
        } else {
            hsla(0., 0., 0., 0.015)
        };
        let specs = self.column_specs(cx);

        for &company_idx in visible {
            let cid = self.companies[company_idx].id.clone();
            let cname = self.companies[company_idx].name.clone();
            let is_checked = self.selected_rows.contains(&cid);
            let toggle_cid = cid.clone();

            let tr_el = div()
                .h(px(40.))
                .w_full()
                .flex_none()
                .border_t_1()
                .border_color(p.line)
                .when(is_checked, |d| d.bg(selected_tint))
                .when(!is_checked, |d| d.hover(move |s| s.bg(row_hover)))
                .flex()
                .items_center()
                .child(
                    div()
                        .w(px(38.))
                        .h_full()
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(self.render_checkbox(
                            pro_eid(&self.id, format!("chk-{cid}")),
                            format!("Select {cname}").into(),
                            is_checked,
                            false,
                            cx,
                            move |this, _, cx| {
                                if let Some(pos) =
                                    this.selected_rows.iter().position(|x| x == &toggle_cid)
                                {
                                    this.selected_rows.remove(pos);
                                } else {
                                    this.selected_rows.push(toggle_cid.clone());
                                }
                                cx.notify();
                            },
                        )),
                );

            let mut cells_wrap = div().flex_1().min_w_0().h_full().flex().items_center();
            for (col_idx, &(_, weight, _)) in specs.iter().enumerate() {
                let is_active = self.active_cell == Some((company_idx, col_idx));
                let is_editing = self.editing_cell == Some((company_idx, col_idx));
                let cell_content = if is_editing {
                    if let Some(ed) = self.cell_editor.clone() {
                        div()
                            .w_full()
                            .h_full()
                            .flex()
                            .items_center()
                            .child(ed)
                            .into_any_element()
                    } else {
                        self.render_cell_content(company_idx, col_idx, cx)
                    }
                } else {
                    self.render_cell_content(company_idx, col_idx, cx)
                };

                cells_wrap = cells_wrap.child(
                    div()
                        .id(pro_eid(&self.id, format!("cell-{cid}-{col_idx}")))
                        .role(Role::Cell)
                        .relative()
                        .w(relative(weight))
                        .min_w_0()
                        .h_full()
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .text_size(px(13.))
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, ev: &MouseDownEvent, w, cx| {
                                if this.editing_cell == Some((company_idx, col_idx)) {
                                    return;
                                }
                                if this.editing_cell.is_some() {
                                    this.commit_cell_edit(cx);
                                }
                                if ev.click_count >= 2 {
                                    this.start_cell_interaction(company_idx, col_idx, w, cx);
                                } else {
                                    this.active_cell = Some((company_idx, col_idx));
                                    this.picker_cell = None;
                                    cx.notify();
                                }
                            }),
                        )
                        .child(cell_content)
                        .when(is_active, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .top(px(1.))
                                    .right(px(1.))
                                    .bottom(px(1.))
                                    .left(px(1.))
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(p.blue),
                            )
                        }),
                );
            }

            table = table.child(tr_el.child(cells_wrap));
            if let Some((pr, _, kind)) = self.picker_cell
                && pr == company_idx
            {
                table = table.child(self.render_picker_bar(company_idx, kind, cx));
            }
        }

        // Table footer: `+ Add row` / bulk actions + summary
        let sel_n = self.selected_rows.len();
        let add_row_label = match self.active_view {
            CrmView::Contacts => tr(cx, "Add contact", "添加联系人"),
            CrmView::Deals => tr(cx, "Add deal", "添加商机"),
            _ => tr(cx, "Add row", "添加行"),
        };

        let left_foot = if sel_n > 0 {
            div()
                .pl(px(8.))
                .flex()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .mr(px(6.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(format!("{sel_n} {}", tr(cx, "selected", "项已选"))),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "bulk-clear"),
                    tr(cx, "Clear", "取消选择"),
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.selected_rows.clear();
                        cx.notify();
                    },
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "bulk-delete"),
                    tr(cx, "Delete", "删除"),
                    Some(ProIcon::Trash),
                    ProBtnStyle::Danger,
                    true,
                    cx,
                    |this, _, cx| this.delete_selected_rows(cx),
                ))
        } else {
            div().child(activate(
                div()
                    .id(pro_eid(&self.id, "add-row"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .child(pro_icon(ProIcon::Plus, 14., p.muted))
                    .child(add_row_label),
                cx,
                |this, w, cx| this.add_quick_row(w, cx),
            ))
        };

        let right_summary = if sel_n > 0 {
            let sum: u64 = self
                .companies
                .iter()
                .filter(|c| self.selected_rows.contains(&c.id))
                .map(|c| c.value)
                .sum();
            format!("${} {}", format_thousands(sum), tr(cx, "total", "合计"))
        } else {
            tr(cx, "Double-click a cell to edit", "双击单元格即可编辑").to_string()
        };

        table.child(
            div()
                .min_h(px(44.))
                .w_full()
                .pl(px(8.))
                .pr(px(16.))
                .py(px(8.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .child(left_foot)
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .whitespace_nowrap()
                        .child(right_summary),
                ),
        )
    }

    fn render_pipeline_board(&self, visible: &[usize], cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let stages = [
            CrmStage::New,
            CrmStage::Qualified,
            CrmStage::Proposal,
            CrmStage::Negotiation,
            CrmStage::Won,
        ];

        let mut board = div()
            .p(px(12.))
            .min_w(px(820.))
            .flex()
            .items_start()
            .gap(px(10.));

        for st in stages {
            let col_items: Vec<usize> = visible
                .iter()
                .copied()
                .filter(|&i| self.companies[i].stage == st)
                .collect();
            let stage_sum: u64 = col_items.iter().map(|&i| self.companies[i].value).sum();

            let mut cards = div().flex().flex_col().gap(px(8.));
            for idx in col_items {
                let c = &self.companies[idx];
                let cid = c.id.clone();
                let is_sel = self.inspector_id.as_ref() == Some(&cid);
                cards = cards.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("bd-card-{cid}")))
                        .role(Role::Button)
                        .tab_index(0)
                        .p(px(10.))
                        .rounded(px(9.))
                        .bg(p.panel)
                        .border_1()
                        .border_color(if is_sel { p.blue } else { p.line })
                        .hover(move |s| s.border_color(p.line_strong))
                        .flex()
                        .flex_col()
                        .gap(px(7.))
                        .cursor_pointer()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .child(pro_app_tile(&c.name, 20., cx))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(c.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.strong)
                                        .child(format!("${}", format_thousands(c.value))),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(6.))
                                .child(render_dtc_tag(
                                    c.category.clone(),
                                    category_tone(&c.category),
                                    cx,
                                ))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(p.muted)
                                        .child(format!("{}%", c.stage.probability())),
                                ),
                        )
                        .child(
                            div()
                                .pt(px(4.))
                                .border_t_1()
                                .border_color(p.line_soft)
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(5.))
                                        .child(pro_person_avatar(&c.owner, 16., false, None, cx))
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .text_color(p.text)
                                                .child(c.owner.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(p.faint)
                                        .child(c.last_contact.clone()),
                                ),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.open_inspector(cid.clone(), cx);
                    },
                ));
            }

            board = board.child(
                div()
                    .flex_1()
                    .min_w(px(152.))
                    .p(px(8.))
                    .rounded(px(10.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .px(px(2.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(render_dtc_tag(st.label(cx), st.tag_tone(), cx))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(format!("${}", format_thousands(stage_sum))),
                            ),
                    )
                    .child(cards),
            );
        }

        board
    }

    fn render_empty_for_view(&self, cx: &mut Context<Self>) -> Div {
        let view = self.active_view;
        let view_id: SharedString = view.as_id().into();
        let (icon, title, caption, cta_label) = match view {
            CrmView::Companies => (
                ProIcon::Building,
                tr(cx, "No companies yet", "暂无客户记录"),
                tr(
                    cx,
                    "Add your first company to track pipeline value, owners and touchpoints.",
                    "添加首家客户以跟进商机金额、负责人与最近互动。",
                ),
                tr(cx, "Add company", "添加公司"),
            ),
            CrmView::Deals => (
                ProIcon::Coins,
                tr(cx, "No deals yet", "暂无商机"),
                tr(
                    cx,
                    "Create a deal to forecast revenue across qualification, proposal and close.",
                    "创建商机以预测从线索验证到成交的各阶段收入。",
                ),
                tr(cx, "Add deal", "添加商机"),
            ),
            CrmView::Contacts => (
                ProIcon::Users,
                tr(cx, "No contacts yet", "暂无联系人"),
                tr(
                    cx,
                    "Add decision-makers and champions linked to your target accounts.",
                    "添加与目标客户关联的决策人与关键联系人。",
                ),
                tr(cx, "Add contact", "添加联系人"),
            ),
            CrmView::Renewals => (
                ProIcon::Refresh,
                tr(cx, "No renewals yet", "暂无续约合同"),
                tr(
                    cx,
                    "Track upcoming contract renewals, expansion potential and account health.",
                    "跟踪即将到期的续约合同、增购空间与客户健康度。",
                ),
                tr(cx, "Add renewal", "添加续约"),
            ),
        };

        div()
            .flex_1()
            .min_h(px(320.))
            .flex()
            .items_center()
            .justify_center()
            .child(pro_empty_state(
                icon,
                title,
                caption,
                Some((
                    pro_eid(&self.id, "empty-add"),
                    cta_label.into(),
                    Box::new(move |this: &mut Self, w, cx| {
                        cx.emit(CrmEvent::AddRequested(view_id.clone()));
                        this.add_quick_row(w, cx);
                    }),
                )),
                cx,
            ))
    }

    fn render_inspector(
        &mut self,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let target_id = self.inspector_id.clone()?;
        let comp = self.companies.iter().find(|c| c.id == target_id)?.clone();
        let p = ProPalette::get(cx);
        let note_ed = self.note_editor.clone();
        let cid_stage = comp.id.clone();
        let cid_note = comp.id.clone();

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
                    .items_center()
                    .gap(px(10.))
                    .child(pro_app_tile(&comp.name, 34., cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(comp.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(format!("{} · {}", comp.domain, comp.category)),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.strong)
                            .child(format!("${}", format_thousands(comp.value))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Pipeline stage", "商机阶段")),
                    )
                    .child({
                        let insp_st_sensor = pro_selection_sensor();
                        div()
                            .relative()
                            .flex()
                            .flex_wrap()
                            .gap(px(5.))
                            .child(pro_pill_indicator(
                                pro_eid(&self.id, "insp-st-pill"),
                                comp.stage.as_str(),
                                insp_st_sensor.clone(),
                                7.,
                                transparent_black(),
                                Some(p.blue),
                                false,
                            ))
                            .children(CrmStage::ALL.into_iter().map(|st| {
                                let on = comp.stage == st;
                                let cid = cid_stage.clone();
                                activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("insp-st-{}", st.as_str())))
                                        .role(Role::Button)
                                        .tab_index(0)
                                        .relative()
                                        .p(px(2.))
                                        .rounded(px(7.))
                                        .border_1()
                                        .border_color(transparent_black())
                                        .when(on, |d| d.child(pro_mark_selected(&insp_st_sensor)))
                                        .cursor_pointer()
                                        .child(render_dtc_tag(st.label(cx), st.tag_tone(), cx)),
                                    cx,
                                    move |this, _, cx| {
                                        if let Some(c) =
                                            this.companies.iter_mut().find(|x| x.id == cid)
                                        {
                                            c.stage = st;
                                            cx.emit(CrmEvent::StageChanged {
                                                company_id: cid.clone(),
                                                stage: st,
                                            });
                                            cx.notify();
                                        }
                                    },
                                )
                            }))
                    }),
            )
            .child(
                div()
                    .p(px(11.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(9.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                                cx,
                                "Primary contact",
                                "主要联系人",
                            )))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(pro_person_avatar(&comp.contact, 18., false, None, cx))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(comp.contact.clone()),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                                cx,
                                "Account owner",
                                "客户负责人",
                            )))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(pro_person_avatar(&comp.owner, 18., false, None, cx))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(p.ink)
                                            .child(comp.owner.clone()),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                                cx,
                                "Next activity",
                                "下一步跟进",
                            )))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(p.ink)
                                    .child(comp.next_step.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                                        cx,
                                        "Win probability",
                                        "赢单概率",
                                    )))
                                    .child(pro_badge(
                                        format!("{}%", comp.stage.probability()),
                                        ProTone::Blue,
                                        true,
                                        cx,
                                    )),
                            )
                            .child(pro_spring_bar(
                                pro_eid(&self.id, format!("win-prob-{}", comp.id)),
                                (comp.stage.probability() as f32 / 100.0).clamp(0.0, 1.0),
                                5.,
                                2.5,
                                p.chip,
                                p.blue,
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Activity & notes", "跟进动态与备注")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(30.))
                                    .px(px(9.))
                                    .rounded(px(7.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .items_center()
                                    .when_some(note_ed.clone(), |d, ed| {
                                        d.child(div().flex_1().min_w_0().child(ed))
                                    }),
                            )
                            .child(pro_btn(
                                pro_eid(&self.id, "log-note-btn"),
                                tr(cx, "Log", "记录"),
                                None,
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                move |this, w, cx| {
                                    if let Some(ed) = this.note_editor.clone() {
                                        let txt = ed.read(cx).value().trim().to_string();
                                        if !txt.is_empty()
                                            && let Some(c) =
                                                this.companies.iter_mut().find(|x| x.id == cid_note)
                                        {
                                            c.notes.insert(
                                                0,
                                                CrmActivityNote::new(
                                                    "Alex Morgan",
                                                    txt.clone(),
                                                    "Just now",
                                                ),
                                            );
                                            c.last_contact = "Just now".into();
                                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                                            cx.emit(CrmEvent::NoteAdded {
                                                company_id: cid_note.clone(),
                                                note: txt.into(),
                                            });
                                            cx.notify();
                                        }
                                    }
                                },
                            )),
                    )
                    .children(comp.notes.into_iter().enumerate().map(|(ni, note)| {
                        div()
                            .id(pro_eid(&self.id, format!("note-{ni}")))
                            .p(px(10.))
                            .rounded(px(9.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .child(pro_person_avatar(
                                                &note.author,
                                                16.,
                                                false,
                                                None,
                                                cx,
                                            ))
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.ink)
                                                    .child(note.author),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(p.faint)
                                            .child(note.timestamp),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .text_color(p.text)
                                    .child(note.summary),
                            )
                    })),
            );

        let animated_body = pro_view_transition(pro_eid(&self.id, "crm-insp-vtx"), target_id, body);
        Some((comp.name, ProIcon::Building, animated_body))
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let width = self.measured_width.get();
        let compact = width < 640.;
        let measure_cell = self.measured_width.clone();

        // Top breadcrumb header: `Folder Sales / Head of Sales v [Star]`
        let starred = self.starred;
        let top_bar = div()
            .h(px(48.))
            .pl(px(if compact { 46. } else { 20. }))
            .pr(px(16.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .text_color(p.muted)
                            .whitespace_nowrap()
                            .child(pro_icon(ProIcon::Folder, 15., p.muted))
                            .child(tr(cx, "Sales", "商务"))
                            .child(div().text_color(p.faint).child("/"))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "saved-view-btn"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .ml(px(-4.))
                                    .h(px(26.))
                                    .px(px(6.))
                                    .rounded(px(7.))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(p.hover))
                                    .child(self.saved_view.clone())
                                    .child(pro_icon(ProIcon::ChevronDown, 13., p.muted)),
                                cx,
                                |this, _, cx| {
                                    this.saved_view_menu_open = !this.saved_view_menu_open;
                                    this.all_views_menu_open = false;
                                    cx.notify();
                                },
                            )),
                    )
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "fav-star"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Add to favorites", "添加到收藏"))
                            .tab_index(0)
                            .size(px(26.))
                            .rounded(px(7.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover))
                            .child(if starred {
                                pro_svg("0 0 24 24", ICON_STAR_FILLED, 15., p.amber)
                            } else {
                                pro_icon(ProIcon::Star, 15., p.muted)
                            }),
                        cx,
                        |this, _, cx| {
                            this.starred = !this.starred;
                            cx.emit(CrmEvent::FavoriteToggled(this.starred));
                            cx.notify();
                        },
                    )),
            )
            .when(
                !self.undo_stack.is_empty() || !self.redo_stack.is_empty(),
                |d| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "undo-btn"),
                                tr(cx, "Undo (⌘Z)", "撤销 (⌘Z)"),
                                ProIcon::Undo,
                                26.,
                                false,
                                cx,
                                |this, _, cx| this.undo(cx),
                            ))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "redo-btn"))
                                    .role(Role::Button)
                                    .aria_label(tr(cx, "Redo (⌘⇧Z)", "重做 (⌘⇧Z)"))
                                    .tab_index(0)
                                    .size(px(26.))
                                    .rounded(px(7.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(p.hover))
                                    .child(pro_svg("0 0 24 24", ICON_REDO, 15., p.muted)),
                                cx,
                                |this, _, cx| this.redo(cx),
                            )),
                    )
                },
            );

        // Open views tab bar: `[New view] [All views] | [Company CRM] [Deals] [Contacts] [Renewals]`
        let active_view = self.active_view;
        let tabs_bar = div()
            .id(pro_eid(&self.id, "tabs-scroll"))
            .overflow_x_scroll()
            .px(px(if compact { 8. } else { 14. }))
            .pb(px(12.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(4.))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "new-view-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "New view", "新建视图"))
                    .tab_index(0)
                    .size(px(28.))
                    .flex_none()
                    .rounded(px(8.))
                    .bg(if self.composer_open {
                        p.chip
                    } else {
                        transparent_black()
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(pro_svg("0 0 24 24", ICON_NEW_VIEW, 16., p.muted)),
                cx,
                |this, w, cx| {
                    this.composer_open = !this.composer_open;
                    if this.composer_open
                        && let Some(ed) = this.new_name_editor.clone()
                    {
                        ed.update(cx, |e, cx| e.focus(w, cx));
                    }
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "all-views-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "All views", "全部视图与筛选"))
                    .tab_index(0)
                    .size(px(28.))
                    .flex_none()
                    .rounded(px(8.))
                    .bg(if self.toolbar_open || self.all_views_menu_open {
                        p.chip
                    } else {
                        transparent_black()
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(pro_svg("0 0 24 24", ICON_ALL_VIEWS, 16., p.muted)),
                cx,
                |this, _, cx| {
                    this.all_views_menu_open = !this.all_views_menu_open;
                    this.saved_view_menu_open = false;
                    cx.notify();
                },
            ))
            .child(div().w(px(1.)).h(px(18.)).mx(px(6.)).flex_none().bg(p.line))
            .child({
                let view_sensor = pro_selection_sensor();
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "view-tabs-pill"),
                        active_view.as_id(),
                        view_sensor.clone(),
                        9.,
                        p.chip,
                        None,
                        false,
                    ))
                    .children(CrmView::ALL.into_iter().map(|v| {
                        let selected = v == active_view;
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("tab-{}", v.as_id())))
                                .role(Role::Tab)
                                .tab_index(0)
                                .relative()
                                .h(px(32.))
                                .pl(px(10.))
                                .pr(px(12.))
                                .flex_none()
                                .rounded(px(9.))
                                .when(selected, |d| d.child(pro_mark_selected(&view_sensor)))
                                .when(!selected, |d| {
                                    d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                                })
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .text_size(px(13.))
                                .font_weight(if selected {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if selected { p.ink } else { p.muted })
                                .whitespace_nowrap()
                                .cursor_pointer()
                                .child(Self::render_tab_badge(v, cx))
                                .child(v.label(cx)),
                            cx,
                            move |this, _, cx| {
                                this.active_view = v;
                                this.editing_cell = None;
                                this.picker_cell = None;
                                this.active_cell = None;
                                cx.emit(CrmEvent::ViewChanged(v));
                                cx.notify();
                            },
                        )
                    }))
            });

        let show_toolbar = self.active_view == CrmView::Deals
            || self.toolbar_open
            || self.display_mode == CrmDisplayMode::Board
            || self.stage_filter != CrmStageFilter::All;

        let toolbar_div = self.render_kpi_and_toolbar(compact, cx);
        let toolbar_fold = pro_fold(
            pro_eid(&self.id, "crm-toolbar-fold"),
            show_toolbar,
            if self.active_view == CrmView::Deals {
                114.
            } else {
                44.
            },
            0.,
            move || toolbar_div.into_any_element(),
        );
        let composer_div = self.render_composer(cx);
        let composer_fold = pro_fold(
            pro_eid(&self.id, "crm-composer-fold"),
            self.composer_open,
            56.,
            0.,
            move || composer_div.into_any_element(),
        );

        let query = self
            .search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        let visible =
            filter_and_sort_companies(&self.companies, self.stage_filter, &query, self.sort);

        let table_inner: AnyElement = if self.loading {
            self.render_skeleton_table(w, cx).into_any_element()
        } else if self.empty || self.companies.is_empty() {
            self.render_empty_for_view(cx).into_any_element()
        } else if visible.is_empty() {
            div()
                .flex_1()
                .min_h(px(280.))
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Filter,
                    tr(cx, "No matching records", "没有匹配的客户记录"),
                    tr(
                        cx,
                        "Try clearing your stage filter or search query.",
                        "请可改清除阶段筛选或搜索关键词。",
                    ),
                    Some((
                        pro_eid(&self.id, "reset-filters"),
                        tr(cx, "Reset filters", "重置筛选").into(),
                        Box::new(|this: &mut Self, w, cx| {
                            this.stage_filter = CrmStageFilter::All;
                            if let Some(ed) = &this.search_editor {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else if self.display_mode == CrmDisplayMode::Board {
            self.render_pipeline_board(&visible, cx).into_any_element()
        } else {
            self.render_data_table(&visible, cx).into_any_element()
        };

        let main_vtx_key = format!(
            "{}-{:?}-{:?}-{}-{}",
            self.active_view.as_id(),
            self.display_mode,
            self.stage_filter,
            self.loading,
            self.empty
        );
        let table_vtx =
            pro_view_transition(pro_eid(&self.id, "crm-main-vtx"), main_vtx_key, table_inner);

        let mut root = div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .relative()
            .size_full()
            .min_h_0()
            .pb(px(if compact { 6. } else { 12. }))
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
            .child(top_bar)
            .child(tabs_bar)
            .child(toolbar_fold)
            .child(composer_fold)
            .child(
                div()
                    .id(pro_eid(&self.id, "table-card"))
                    .role(Role::TabPanel)
                    .flex_1()
                    .min_h_0()
                    .mx(px(if compact { 6. } else { 12. }))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .overflow_y_scroll()
                    .child(
                        div()
                            .id(pro_eid(&self.id, "table-xscroll"))
                            .overflow_x_scroll()
                            .w_full()
                            .child(table_vtx),
                    ),
            );

        // Saved-view dropdown popover (`Head of Sales` menu)
        {
            let cur_saved = self.saved_view.clone();
            let sv_sensor = pro_selection_sensor();
            let saved_pop = div()
                .id(pro_eid(&self.id, "saved-view-pop"))
                .role(Role::Menu)
                .absolute()
                .left(px(58.))
                .w(px(196.))
                .p(px(4.))
                .rounded(px(11.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "sv-pill"),
                    cur_saved.clone(),
                    sv_sensor.clone(),
                    7.,
                    p.select,
                    None,
                    false,
                ))
                .child(
                    div()
                        .px(px(10.))
                        .py(px(5.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.faint)
                        .child(tr(cx, "Saved views", "已保存视图")),
                )
                .children(SAVED_VIEWS.into_iter().map(|sv| {
                    let checked = cur_saved.as_ref() == sv;
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("sv-{sv}")))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .relative()
                            .h(px(30.))
                            .px(px(10.))
                            .rounded(px(7.))
                            .when(checked, |d| d.child(pro_mark_selected(&sv_sensor)))
                            .when(!checked, |d| d.hover(move |s| s.bg(p.hover)))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(13.))
                            .font_weight(if checked {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if checked { p.ink } else { p.text })
                            .cursor_pointer()
                            .child(sv)
                            .when(checked, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                        cx,
                        move |this, _, cx| {
                            this.saved_view = sv.into();
                            this.saved_view_menu_open = false;
                            cx.notify();
                        },
                    )
                }));
            root = root.child(pro_presence(
                pro_eid(&self.id, "saved-view-pres"),
                self.saved_view_menu_open,
                0.18,
                0.12,
                move |a| {
                    saved_pop
                        .top(px(42. - 5. * (1. - a)))
                        .opacity(a)
                        .into_any_element()
                },
            ));
        }

        // All-views & layout menu popover
        {
            let all_views_pop = div()
                .id(pro_eid(&self.id, "all-views-pop"))
                .role(Role::Menu)
                .absolute()
                .left(px(36.))
                .w(px(220.))
                .p(px(4.))
                .rounded(px(11.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(
                    div()
                        .px(px(10.))
                        .py(px(5.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.faint)
                        .child(tr(cx, "Display & filters", "显示与筛选")),
                )
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "av-toggle-toolbar"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(tr(cx, "Stage filters & search", "阶段筛选与搜索"))
                        .when(self.toolbar_open, |d| {
                            d.child(pro_icon(ProIcon::Check, 13., p.ink))
                        }),
                    cx,
                    |this, _, cx| {
                        this.toolbar_open = !this.toolbar_open;
                        this.all_views_menu_open = false;
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "av-mode-table"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(tr(cx, "Spreadsheet table", "电子表格视图"))
                        .when(self.display_mode == CrmDisplayMode::Table, |d| {
                            d.child(pro_icon(ProIcon::Check, 13., p.ink))
                        }),
                    cx,
                    |this, _, cx| {
                        this.display_mode = CrmDisplayMode::Table;
                        this.all_views_menu_open = false;
                        cx.emit(CrmEvent::DisplayModeChanged(CrmDisplayMode::Table));
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "av-mode-board"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(tr(cx, "Pipeline board", "商机管道看板"))
                        .when(self.display_mode == CrmDisplayMode::Board, |d| {
                            d.child(pro_icon(ProIcon::Check, 13., p.ink))
                        }),
                    cx,
                    |this, _, cx| {
                        this.display_mode = CrmDisplayMode::Board;
                        this.all_views_menu_open = false;
                        cx.emit(CrmEvent::DisplayModeChanged(CrmDisplayMode::Board));
                        cx.notify();
                    },
                ));
            root = root.child(pro_presence(
                pro_eid(&self.id, "all-views-pres"),
                self.all_views_menu_open,
                0.18,
                0.12,
                move |a| {
                    all_views_pop
                        .top(px(80. - 5. * (1. - a)))
                        .opacity(a)
                        .into_any_element()
                },
            ));
        }

        root
    }
}

impl Render for CrmScreen {
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
        CrmStage, CrmStageFilter, SortDir, default_companies, filter_and_sort_companies,
        format_thousands, parse_currency_u64, pipeline_summary,
    };

    #[test]
    fn stage_filtering_search_and_pipeline_metrics() {
        let companies = default_companies();
        assert_eq!(companies.len(), 10);

        let summary = pipeline_summary(&companies);
        assert_eq!(summary.total_pipeline, 426_100);
        assert_eq!(summary.closed_won, 86_000);
        assert_eq!(summary.active_count, 8);

        let negotiation = filter_and_sort_companies(
            &companies,
            CrmStageFilter::Stage(CrmStage::Negotiation),
            "",
            None,
        );
        assert_eq!(negotiation.len(), 2);
        assert_eq!(companies[negotiation[0]].name.as_ref(), "Notion");
        assert_eq!(companies[negotiation[1]].name.as_ref(), "Webflow");

        let by_owner = filter_and_sort_companies(
            &companies,
            CrmStageFilter::All,
            "mara",
            Some((4, SortDir::Desc)),
        );
        assert_eq!(by_owner.len(), 3);
        assert_eq!(companies[by_owner[0]].name.as_ref(), "Figma");
        assert_eq!(companies[by_owner[1]].name.as_ref(), "Notion");
        assert_eq!(companies[by_owner[2]].name.as_ref(), "Airtable");
    }

    #[test]
    fn currency_parsing_and_stage_probabilities() {
        assert_eq!(parse_currency_u64("$124,500"), Some(124_500));
        assert_eq!(parse_currency_u64("   "), None);
        assert_eq!(format_thousands(124_500), "124,500");
        assert_eq!(format_thousands(9_800), "9,800");
        assert_eq!(CrmStage::parse("Closed Won"), Some(CrmStage::Won));
        assert_eq!(CrmStage::Negotiation.probability(), 80);
    }
}
