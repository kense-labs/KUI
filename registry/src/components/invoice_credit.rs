//! Tactile stacked-paper invoice and credit note block with live rolling line adjustments.
//!
//! Stacks an `Invoice` sheet and its `Credit note` sheet like physical paper; users can
//! flip between documents via the segmented switch or peeking back sheet, toggle lines to
//! credit, edit credited amounts inline, watch tax and totals roll live, and issue or undo.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_check_draw, pro_eid, pro_empty_state, pro_fold,
        pro_icon_btn, pro_mark_selected, pro_mix_color, pro_pill_indicator, pro_popover_presence,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_swap_label, pro_tactile,
        pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        editor::{EditEvent, Editor, InputMode},
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// Active front sheet in [`InvoiceCreditBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BillingSheetView {
    /// Show the original customer invoice in front.
    #[default]
    Invoice,
    /// Show the adjustable credit note in front.
    CreditNote,
}

impl BillingSheetView {
    fn key(self) -> &'static str {
        match self {
            Self::Invoice => "invoice",
            Self::CreditNote => "credit",
        }
    }
}

/// Seller (`From`) or buyer (`To`) address block displayed on an invoice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvoiceParty {
    /// Legal company or individual name.
    pub name: SharedString,
    /// Street, city, postal code, and country lines.
    pub lines: Vec<SharedString>,
}

impl InvoiceParty {
    /// Construct an [`InvoiceParty`] with `name` and address `lines`.
    pub fn new(name: impl Into<SharedString>, lines: Vec<SharedString>) -> Self {
        Self {
            name: name.into(),
            lines,
        }
    }
}

/// One billed item or discount row on [`InvoiceCreditBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct InvoiceLine {
    /// Stable line identifier (for example `"plan"` or `"api"`).
    pub id: SharedString,
    /// Line item description.
    pub name: SharedString,
    /// Billed quantity.
    pub qty: f64,
    /// Unit rate (`< 0.0` renders as a discount; non-finite renders `"—"` and counts as `0.0`).
    pub rate: f64,
    /// Fractional tax rate (for example `0.20` for `20%`).
    pub tax: f64,
    /// Optional section group heading (for example `"Metered usage"`).
    pub group: Option<SharedString>,
}

impl InvoiceLine {
    /// Construct an [`InvoiceLine`].
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        qty: f64,
        rate: f64,
        tax: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            qty,
            rate,
            tax,
            group: None,
        }
    }

    /// Group this line item under a shared section heading on the invoice table.
    pub fn with_group(mut self, group: impl Into<SharedString>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Return whether `qty` and `rate` produce a finite number.
    pub fn has_valid_rate(&self) -> bool {
        self.qty.is_finite() && self.rate.is_finite()
    }

    /// Pre-tax line amount (`0.0` when non-finite).
    pub fn subtotal(&self) -> f64 {
        if self.has_valid_rate() {
            self.qty * self.rate
        } else {
            0.0
        }
    }

    /// Tax amount for a given pre-tax `amount`.
    pub fn tax_for(&self, amount: f64) -> f64 {
        let rate = if self.tax.is_finite() {
            self.tax.max(0.0)
        } else {
            0.0
        };
        amount * rate
    }

    /// Whether this line can be selected for credit (`subtotal() > 0.0`).
    pub fn is_creditable(&self) -> bool {
        self.subtotal() > 0.0
    }
}

/// Per-line credit allocation emitted when issuing a credit note.
#[derive(Clone, Debug, PartialEq)]
pub struct CreditedLine {
    /// Line item id.
    pub line_id: SharedString,
    /// Pre-tax credited amount for this line (`0.0..=line.subtotal()`).
    pub amount: f64,
    /// Proportional tax credited for this line.
    pub tax_amount: f64,
}

/// Typed events emitted by [`InvoiceCreditBlock`].
#[derive(Clone, Debug, PartialEq)]
pub enum InvoiceCreditEvent {
    /// User flipped between the `Invoice` and `Credit note` sheets.
    ViewChanged(BillingSheetView),
    /// User toggled or edited a line item's credited pre-tax amount.
    LineAdjusted {
        line_id: SharedString,
        checked: bool,
        amount: f64,
    },
    /// User issued the credit note.
    Issued {
        credit_no: SharedString,
        credits: Vec<CreditedLine>,
        subtotal: f64,
        tax: f64,
        total: f64,
    },
    /// User undid / reopened an issued credit note.
    Undone(SharedString),
    /// User clicked the `PDF` export action.
    PdfRequested(BillingSheetView),
}

#[derive(Clone)]
struct LineCreditState {
    line_id: SharedString,
    checked: bool,
    amount: f64,
    max_amount: f64,
    check_slide: Slide,
    editor: Option<Entity<Editor>>,
}

fn resolve_currency(spec: &str) -> (&'static str, &'static str, usize) {
    match spec.trim().to_ascii_uppercase().as_str() {
        "$" | "USD" => ("$", "USD", 2),
        "€" | "EUR" => ("€", "EUR", 2),
        "£" | "GBP" => ("£", "GBP", 2),
        "¥" | "JPY" => ("¥", "JPY", 0),
        _ => ("$", "USD", 2),
    }
}

fn format_with_commas(val: f64, decimals: usize) -> String {
    let abs = if val.is_finite() { val.abs() } else { 0.0 };
    let raw = format!("{abs:.decimals$}");
    let (int_part, frac_part) = match raw.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (raw.as_str(), None),
    };
    let bytes = int_part.as_bytes();
    let mut grouped = String::with_capacity(int_part.len() + int_part.len() / 3);
    for (idx, &b) in bytes.iter().enumerate() {
        if idx > 0 && (bytes.len() - idx) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(char::from(b));
    }
    match frac_part {
        Some(f) => format!("{grouped}.{f}"),
        None => grouped,
    }
}

fn format_money(val: f64, symbol: &str, decimals: usize) -> String {
    let neg = val < -0.000_5;
    let body = format_with_commas(val, decimals);
    if neg {
        format!("-{symbol}{body}")
    } else {
        format!("{symbol}{body}")
    }
}

fn format_qty(qty: f64) -> String {
    if !qty.is_finite() {
        return "—".to_owned();
    }
    if (qty - qty.round()).abs() < 1e-6 {
        format_with_commas(qty, 0)
    } else {
        format_with_commas(qty, 2)
    }
}

fn default_seller() -> InvoiceParty {
    InvoiceParty::new(
        "Lumen Labs, Inc.",
        vec![
            "48 Harbour Street".into(),
            "Bristol".into(),
            "BS1 4RN".into(),
            "United Kingdom".into(),
        ],
    )
}

fn default_buyer() -> InvoiceParty {
    InvoiceParty::new(
        "Northwind Logistics GmbH",
        vec![
            "Speicherstraße 12".into(),
            "Hamburg".into(),
            "20457".into(),
            "Germany".into(),
        ],
    )
}

fn default_invoice_lines() -> Vec<InvoiceLine> {
    vec![
        InvoiceLine::new("plan", "Growth plan", 1.0, 1250.0, 0.20),
        InvoiceLine::new("seats", "Additional seats", 12.0, 40.0, 0.20),
        InvoiceLine::new("api", "API calls (per 1k)", 18_420.0, 0.42, 0.20)
            .with_group("Metered usage"),
    ]
}

#[derive(IntoElement)]
struct AddedLineFold {
    id: ElementId,
    animate_in: bool,
    content: AnyElement,
}

impl RenderOnce for AddedLineFold {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.animate_in {
            return div()
                .id(self.id)
                .w_full()
                .child(self.content)
                .into_any_element();
        }
        let st = w.use_keyed_state(pro_eid(&self.id, "lfold"), cx, |_, inner_cx| {
            let mut slide = Slide::new(0.);
            slide.duration(1., 0.25, inner_cx);
            (slide, Rc::new(Cell::new(px(38.))))
        });
        let (amt, h_cell) = st.update(cx, |s, cx| (s.0.get(w, cx).clamp(0., 1.), s.1.clone()));
        let h = h_cell.get();
        div()
            .id(self.id)
            .w_full()
            .flex_none()
            .when(amt < 0.998, |v| v.h(h * amt).overflow_hidden())
            .opacity(amt)
            .child(
                div()
                    .relative()
                    .top(px(5. * (1. - amt)))
                    .w_full()
                    .flex_none()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first()
                            && b.size.height > px(0.)
                        {
                            h_cell.set(b.size.height);
                        }
                    })
                    .child(self.content),
            )
            .into_any_element()
    }
}

type SheetClickCallback = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
struct BackSheetPeek {
    id: ElementId,
    aria: SharedString,
    top: f32,
    inset: f32,
    rest_bg: Hsla,
    hover_bg: Hsla,
    border: Hsla,
    on_click: SheetClickCallback,
}

impl RenderOnce for BackSheetPeek {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let st = w.use_keyed_state(pro_eid(&self.id, "peek-hov"), cx, |_, _| Slide::snappy(0.));
        let hov = st.update(cx, |s, cx| s.get(w, cx).clamp(0., 1.));
        let bg = pro_mix_color(self.rest_bg, self.hover_bg, hov);
        let on_click = self.on_click;
        div()
            .id(self.id)
            .role(Role::Button)
            .aria_label(self.aria)
            .tab_index(-1)
            .absolute()
            .top(px(self.top - 2.5 * hov))
            .left(px(self.inset))
            .right(px(self.inset))
            .h(px(48.))
            .rounded(px(12.))
            .bg(bg)
            .border_1()
            .border_color(self.border)
            .cursor_pointer()
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.set(if *inside { 1. } else { 0. }, cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_click(move |ev, w, cx| (on_click)(ev, w, cx))
    }
}

fn addon_line_pool() -> [(&'static str, f64, f64); 4] {
    [
        ("Dedicated SSO & SCIM add-on", 1.0, 240.0),
        ("Priority SLA support", 1.0, 350.0),
        ("Webhook burst overage (per 10k)", 6.0, 18.0),
        ("Warehouse sync connector", 1.0, 190.0),
    ]
}

/// Stacked-paper `Invoice` and `Credit note` billing block with spring sheet swap and live rolling totals.
pub struct InvoiceCreditBlock {
    id: ElementId,
    focus: FocusHandle,
    seller: InvoiceParty,
    buyer: InvoiceParty,
    invoice_no: SharedString,
    credit_no: SharedString,
    invoice_date: SharedString,
    due_date: SharedString,
    period: SharedString,
    currency: SharedString,
    lines: Vec<InvoiceLine>,
    credit_states: Vec<LineCreditState>,
    added_line_ids: Vec<SharedString>,
    addon_seq: usize,
    view: BillingSheetView,
    sheet_slide: Slide,
    issued: bool,
    loading: bool,
    empty: bool,
    toast: Option<(SharedString, Instant)>,
    clock: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl EventEmitter<InvoiceCreditEvent> for InvoiceCreditBlock {}

impl InvoiceCreditBlock {
    /// Construct an [`InvoiceCreditBlock`] with `INV-2041` (`$11,359.68`) and `CN-0117` stacked behind it.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let lines = default_invoice_lines();
        let credit_states = Self::build_credit_states(&lines);

        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            seller: default_seller(),
            buyer: default_buyer(),
            invoice_no: "INV-2041".into(),
            credit_no: "CN-0117".into(),
            invoice_date: "1 Oct 2026".into(),
            due_date: "31 Oct 2026".into(),
            period: "1 Sep – 30 Sep 2026".into(),
            currency: "$".into(),
            lines,
            credit_states,
            added_line_ids: Vec::new(),
            addon_seq: 0,
            view: BillingSheetView::Invoice,
            sheet_slide: Slide::new(0.),
            issued: false,
            loading: false,
            empty: false,
            toast: None,
            clock: Instant::now(),
            measured_width: Rc::new(Cell::new(894.)),
        }
    }

    fn build_credit_states(lines: &[InvoiceLine]) -> Vec<LineCreditState> {
        lines
            .iter()
            .filter(|l| l.is_creditable())
            .map(|l| {
                let max_amt = l.subtotal().max(0.0);
                LineCreditState {
                    line_id: l.id.clone(),
                    checked: true,
                    amount: max_amt,
                    max_amount: max_amt,
                    check_slide: Slide::new(1.),
                    editor: None,
                }
            })
            .collect()
    }

    /// Override the `From` (`seller`) party block.
    pub fn with_seller(mut self, seller: InvoiceParty) -> Self {
        self.seller = seller;
        self
    }

    /// Override the `To` (`buyer`) party block.
    pub fn with_buyer(mut self, buyer: InvoiceParty) -> Self {
        self.buyer = buyer;
        self
    }

    /// Set the invoice document number (`"INV-2041"` by default).
    pub fn with_invoice_no(mut self, invoice_no: impl Into<SharedString>) -> Self {
        self.invoice_no = invoice_no.into();
        self
    }

    /// Set the credit note document number (`"CN-0117"` by default).
    pub fn with_credit_no(mut self, credit_no: impl Into<SharedString>) -> Self {
        self.credit_no = credit_no.into();
        self
    }

    /// Set the issue date (`""` omits the line).
    pub fn with_invoice_date(mut self, date: impl Into<SharedString>) -> Self {
        self.invoice_date = date.into();
        self
    }

    /// Set the invoice due date (`""` omits the line).
    pub fn with_due_date(mut self, due_date: impl Into<SharedString>) -> Self {
        self.due_date = due_date.into();
        self
    }

    /// Set the billing period (`""` omits the line).
    pub fn with_period(mut self, period: impl Into<SharedString>) -> Self {
        self.period = period.into();
        self
    }

    /// Set the currency symbol (`"$"`, `"€"`, `"£"`) or ISO code (`"USD"`, `"EUR"`, `"JPY"`).
    pub fn with_currency(mut self, currency: impl Into<SharedString>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Replace the invoice line items and initialize credit allocations to each creditable line's subtotal.
    pub fn with_lines(mut self, lines: Vec<InvoiceLine>) -> Self {
        self.credit_states = Self::build_credit_states(&lines);
        self.lines = lines;
        self
    }

    /// Choose which sheet starts in front (`Invoice` by default).
    pub fn with_view(mut self, view: BillingSheetView) -> Self {
        self.view = view;
        self.sheet_slide
            .reset(if view == BillingSheetView::CreditNote {
                1.
            } else {
                0.
            });
        self
    }

    /// Configure skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently active sheet (`Invoice` or `CreditNote`).
    pub fn view(&self) -> BillingSheetView {
        self.view
    }

    /// Return whether the credit note has been issued.
    pub fn is_issued(&self) -> bool {
        self.issued
    }

    /// Compute `(subtotal, tax, total)` for the original invoice.
    pub fn invoice_totals(&self) -> (f64, f64, f64) {
        let mut sub = 0.0;
        let mut tax = 0.0;
        for line in &self.lines {
            let s = line.subtotal();
            sub += s;
            tax += line.tax_for(s);
        }
        (sub, tax, sub + tax)
    }

    /// Compute `(subtotal, tax, total)` currently selected on the credit note.
    pub fn credit_totals(&self) -> (f64, f64, f64) {
        let mut sub = 0.0;
        let mut tax = 0.0;
        for st in &self.credit_states {
            if !st.checked || st.amount <= 0.0 {
                continue;
            }
            let amt = st.amount.clamp(0.0, st.max_amount);
            sub += amt;
            if let Some(line) = self.lines.iter().find(|l| l.id == st.line_id) {
                tax += line.tax_for(amt);
            }
        }
        (sub, tax, sub + tax)
    }

    /// Switch the front document sheet with a smooth `Slide` spring.
    pub fn set_view(&mut self, next: BillingSheetView, cx: &mut Context<Self>) {
        if self.view == next {
            return;
        }
        self.view = next;
        let target = if next == BillingSheetView::CreditNote {
            1.
        } else {
            0.
        };
        self.sheet_slide.duration(target, 0.40, cx);
        cx.emit(InvoiceCreditEvent::ViewChanged(next));
        cx.notify();
    }

    /// Toggle whether `line_id` is included on the credit note.
    pub fn toggle_credit_line(
        &mut self,
        line_id: &str,
        w: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.issued {
            return false;
        }
        let (_, _, decimals) = resolve_currency(&self.currency);
        let Some(st) = self
            .credit_states
            .iter_mut()
            .find(|s| s.line_id.as_ref() == line_id)
        else {
            return false;
        };
        st.checked = !st.checked;
        if st.checked && st.amount <= 0.0 {
            st.amount = st.max_amount;
            if let (Some(ed), Some(win)) = (&st.editor, w) {
                let formatted = format!("{:.decimals$}", st.amount);
                ed.update(cx, |e, cx| e.set_value(formatted, win, cx));
            }
        }
        st.check_slide
            .duration(if st.checked { 1. } else { 0. }, 0.22, cx);
        let checked = st.checked;
        let amount = if checked { st.amount } else { 0.0 };
        let lid = st.line_id.clone();
        cx.emit(InvoiceCreditEvent::LineAdjusted {
            line_id: lid,
            checked,
            amount,
        });
        cx.notify();
        true
    }

    /// Set the pre-tax credited amount for `line_id`, clamped to `0.0..=line.subtotal()`.
    pub fn set_credit_amount(
        &mut self,
        line_id: &str,
        raw_amount: f64,
        w: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.issued {
            return false;
        }
        let (_, _, decimals) = resolve_currency(&self.currency);
        let Some(st) = self
            .credit_states
            .iter_mut()
            .find(|s| s.line_id.as_ref() == line_id)
        else {
            return false;
        };
        let clamped = if raw_amount.is_finite() {
            raw_amount.clamp(0.0, st.max_amount)
        } else {
            st.amount
        };
        st.amount = clamped;
        let next_checked = clamped > 0.000_5;
        if st.checked != next_checked {
            st.checked = next_checked;
            st.check_slide
                .duration(if next_checked { 1. } else { 0. }, 0.22, cx);
        }
        if let (Some(ed), Some(win)) = (&st.editor, w) {
            let formatted = format!("{clamped:.decimals$}");
            ed.update(cx, |e, cx| e.set_value(formatted, win, cx));
        }
        let lid = st.line_id.clone();
        let checked = st.checked;
        cx.emit(InvoiceCreditEvent::LineAdjusted {
            line_id: lid,
            checked,
            amount: clamped,
        });
        cx.notify();
        true
    }

    /// Issue the drafted credit note if its total is greater than zero.
    pub fn issue_credit_note(&mut self, cx: &mut Context<Self>) -> bool {
        let (subtotal, tax, total) = self.credit_totals();
        if self.issued || total <= 0.000_5 {
            return false;
        }
        self.issued = true;
        let credits: Vec<CreditedLine> = self
            .credit_states
            .iter()
            .filter(|s| s.checked && s.amount > 0.0)
            .map(|s| {
                let amt = s.amount.clamp(0.0, s.max_amount);
                let tax_amt = self
                    .lines
                    .iter()
                    .find(|l| l.id == s.line_id)
                    .map(|l| l.tax_for(amt))
                    .unwrap_or(0.0);
                CreditedLine {
                    line_id: s.line_id.clone(),
                    amount: amt,
                    tax_amount: tax_amt,
                }
            })
            .collect();
        let cno = self.credit_no.clone();
        self.toast = Some((
            if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("已签发贷项通知单 {cno}").into()
            } else {
                format!("Issued credit note {cno}").into()
            },
            Instant::now(),
        ));
        cx.emit(InvoiceCreditEvent::Issued {
            credit_no: cno,
            credits,
            subtotal,
            tax,
            total,
        });
        cx.notify();
        true
    }

    /// Undo / reopen an issued credit note so lines and amounts can be edited again.
    pub fn undo_credit_note(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.issued {
            return false;
        }
        self.issued = false;
        let cno = self.credit_no.clone();
        self.toast = Some((
            tr(
                cx,
                "Reopened credit note for editing",
                "已撤回贷项通知单并恢复编辑",
            )
            .into(),
            Instant::now(),
        ));
        cx.emit(InvoiceCreditEvent::Undone(cno));
        cx.notify();
        true
    }

    /// Toggle skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether loading skeletons are active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether empty state is forced on.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    fn append_addon_line(&mut self, cx: &mut Context<Self>) {
        if self.issued {
            return;
        }
        let pool = addon_line_pool();
        let (name, qty, rate) = pool[self.addon_seq % pool.len()];
        self.addon_seq += 1;
        let lid: SharedString = format!("addon-{}", self.addon_seq).into();
        let line = InvoiceLine::new(lid.clone(), name, qty, rate, 0.20).with_group("Add-ons");
        let max_amt = line.subtotal().max(0.0);
        self.added_line_ids.push(lid.clone());
        self.lines.push(line);
        self.credit_states.push(LineCreditState {
            line_id: lid,
            checked: true,
            amount: max_amt,
            max_amount: max_amt,
            check_slide: Slide::new(1.),
            editor: None,
        });
        self.toast = Some((
            if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("已新增明细项：{name}").into()
            } else {
                format!("Added line item · {name}").into()
            },
            Instant::now(),
        ));
        cx.notify();
    }

    fn ensure_line_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let (_, _, decimals) = resolve_currency(&self.currency);
        for idx in 0..self.credit_states.len() {
            if self.credit_states[idx].editor.is_some() {
                continue;
            }
            let init_val = format!("{:.decimals$}", self.credit_states[idx].amount);
            let lid = self.credit_states[idx].line_id.clone();
            let ed = cx.new(|cx| {
                let mut e = Editor::new("0.00", InputMode::Single, w, cx);
                e.set_value(init_val, w, cx);
                e.font_size = 13.;
                e
            });
            let lid_sub = lid.clone();
            cx.subscribe_in(
                &ed,
                w,
                move |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                    EditEvent::Submitted | EditEvent::Blurred => {
                        let raw = ed_ent.read(cx).value().replace([',', ' '], "");
                        if let Ok(parsed) = raw.parse::<f64>() {
                            this.set_credit_amount(&lid_sub, parsed, Some(w), cx);
                        } else if let Some(st) =
                            this.credit_states.iter().find(|s| s.line_id == lid_sub)
                        {
                            let (_, _, dec) = resolve_currency(&this.currency);
                            let restore = format!("{:.dec$}", st.amount);
                            ed_ent.update(cx, |e, cx| e.set_value(restore, w, cx));
                            cx.notify();
                        }
                        if matches!(ev, EditEvent::Submitted) {
                            this.focus.focus(w, cx);
                        }
                    }
                    EditEvent::Escape => {
                        if let Some(st) = this.credit_states.iter().find(|s| s.line_id == lid_sub) {
                            let (_, _, dec) = resolve_currency(&this.currency);
                            let restore = format!("{:.dec$}", st.amount);
                            ed_ent.update(cx, |e, cx| e.set_value(restore, w, cx));
                        }
                        this.focus.focus(w, cx);
                        cx.notify();
                    }
                    EditEvent::Changed => {
                        let raw = ed_ent.read(cx).value().replace([',', ' '], "");
                        if let Ok(parsed) = raw.parse::<f64>()
                            && let Some(st) =
                                this.credit_states.iter_mut().find(|s| s.line_id == lid_sub)
                        {
                            st.amount = parsed.clamp(0.0, st.max_amount);
                            st.checked = st.amount > 0.000_5;
                            cx.notify();
                        }
                    }
                    _ => {}
                },
            )
            .detach();
            self.credit_states[idx].editor = Some(ed);
        }
    }

    fn render_seller_brand(&self, p: &ProPalette, cx: &App) -> Div {
        div()
            .flex_none()
            .pt(px(4.))
            .flex()
            .items_center()
            .gap(px(7.))
            .child(
                div()
                    .size(px(20.))
                    .rounded_full()
                    .bg(p.accent_soft)
                    .border_1()
                    .border_color(p.accent_info.opacity(0.28))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .children(pro_app_artwork("kense", 12., cx)),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text)
                    .child(self.seller.name.clone()),
            )
    }

    fn render_terms_row(
        label: impl Into<SharedString>,
        value_el: impl IntoElement,
        compact: bool,
        p: &ProPalette,
    ) -> Div {
        div()
            .min_h(px(20.))
            .flex()
            .items_baseline()
            .gap(px(12.))
            .text_size(px(13.))
            .line_height(px(20.))
            .child(
                div()
                    .w(px(if compact { 112. } else { 140. }))
                    .flex_none()
                    .text_color(p.muted)
                    .child(label.into()),
            )
            .child(div().flex_1().min_w_0().text_color(p.ink).child(value_el))
    }

    fn render_invoice_sheet_body(&self, compact: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let (sym, _iso, decimals) = resolve_currency(&self.currency);
        let (inv_sub, inv_tax, inv_total) = self.invoice_totals();
        let (_, _, cred_total) = self.credit_totals();
        let net_due = if self.issued {
            (inv_total - cred_total).max(0.0)
        } else {
            inv_total
        };

        let due_caption: SharedString = if self.issued {
            if net_due <= 0.005 {
                tr(cx, "Credited in full", "已全额冲销").into()
            } else if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!(
                    "冲抵 {} 后剩余应付",
                    format_money(cred_total, sym, decimals)
                )
                .into()
            } else {
                format!(
                    "Balance after {} credit",
                    format_money(cred_total, sym, decimals)
                )
                .into()
            }
        } else if !self.due_date.is_empty() {
            if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("到期日 {}", self.due_date).into()
            } else {
                format!("Due {}", self.due_date).into()
            }
        } else {
            tr(cx, "Invoice summary", "账单摘要").into()
        };

        let head = div()
            .mb(px(if compact { 22. } else { 28. }))
            .flex()
            .when(compact, |d| d.flex_col().gap(px(10.)))
            .when(!compact, |d| d.items_start().justify_between().gap(px(16.)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "inv-hero-total"),
                        net_due,
                        decimals,
                        sym,
                        "",
                        24.,
                        FontWeight::MEDIUM,
                        p.strong,
                    ))
                    .child(
                        div()
                            .mt(px(4.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(pro_swap_label(
                                pro_eid(&self.id, "inv-due-caption-lbl"),
                                due_caption,
                                12.,
                                FontWeight::NORMAL,
                                p.muted,
                            ))
                            .when(self.issued, |d| {
                                d.child(
                                    div()
                                        .h(px(19.))
                                        .px(px(6.))
                                        .rounded(px(4.))
                                        .bg(p.green_soft)
                                        .text_color(p.green_ink)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .flex()
                                        .items_center()
                                        .child(format!(
                                            "{} {}",
                                            tr(cx, "Credited ·", "已冲抵 ·"),
                                            self.credit_no
                                        )),
                                )
                            }),
                    ),
            )
            .child(self.render_seller_brand(&p, cx));

        // Parties: From / To
        let render_party = |heading: &'static str, party: &InvoiceParty| {
            let mut col = div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .mb(px(6.))
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(heading),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(19.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(party.name.clone()),
                );
            for line in &party.lines {
                col = col.child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(19.))
                        .text_color(p.text)
                        .child(line.clone()),
                );
            }
            col
        };

        let parties = div()
            .mb(px(if compact { 22. } else { 26. }))
            .flex()
            .when(compact, |d| d.flex_col().gap(px(16.)))
            .when(!compact, |d| d.gap(px(24.)))
            .child(render_party(tr(cx, "FROM", "开票方"), &self.seller))
            .child(render_party(tr(cx, "TO", "收票方"), &self.buyer));

        // Terms section
        let mut terms_dl = div().flex().flex_col().gap(px(3.));
        let inv_no_el = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(self.invoice_no.clone())
            .when(self.issued, |d| {
                d.child(activate(
                    div()
                        .id(pro_eid(&self.id, "jump-to-cn"))
                        .role(Role::Button)
                        .tab_index(0)
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.accent_info)
                        .cursor_pointer()
                        .child(format!("→ {}", self.credit_no)),
                    cx,
                    |this, _, cx| {
                        this.set_view(BillingSheetView::CreditNote, cx);
                    },
                ))
            });
        terms_dl = terms_dl.child(Self::render_terms_row(
            tr(cx, "Invoice number", "账单编号"),
            inv_no_el,
            compact,
            &p,
        ));
        if !self.invoice_date.is_empty() {
            terms_dl = terms_dl.child(Self::render_terms_row(
                tr(cx, "Issue date", "开票日期"),
                self.invoice_date.clone(),
                compact,
                &p,
            ));
        }
        if !self.due_date.is_empty() {
            terms_dl = terms_dl.child(Self::render_terms_row(
                tr(cx, "Due date", "到期日期"),
                self.due_date.clone(),
                compact,
                &p,
            ));
        }
        if !self.period.is_empty() {
            terms_dl = terms_dl.child(Self::render_terms_row(
                tr(cx, "Billing period", "计费周期"),
                self.period.clone(),
                compact,
                &p,
            ));
        }

        let terms_block = div()
            .mb(px(if compact { 22. } else { 26. }))
            .flex()
            .flex_col()
            .child(
                div()
                    .mb(px(6.))
                    .text_size(px(10.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "TERMS", "账单条款")),
            )
            .child(terms_dl);

        // Line items table
        let table_head = div()
            .h(px(28.))
            .pb(px(6.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_end()
            .gap(px(12.))
            .text_size(px(10.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.muted)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(tr(cx, "DESCRIPTION", "项目明细")),
            )
            .child(
                div()
                    .w(px(58.))
                    .flex_none()
                    .text_right()
                    .child(tr(cx, "QTY", "数量")),
            )
            .when(!compact, |d| {
                d.child(div().w(px(76.)).flex_none().text_right().child(tr(
                    cx,
                    "UNIT PRICE",
                    "单价",
                )))
                .child(
                    div()
                        .w(px(48.))
                        .flex_none()
                        .text_right()
                        .child(tr(cx, "TAX", "税率")),
                )
            })
            .child(
                div()
                    .w(px(86.))
                    .flex_none()
                    .text_right()
                    .child(tr(cx, "AMOUNT", "金额")),
            );

        let mut table_rows = div().pt(px(4.)).flex().flex_col();
        let mut last_group: Option<SharedString> = None;
        for line in &self.lines {
            if let Some(grp) = &line.group
                && last_group.as_ref() != Some(grp)
            {
                last_group = Some(grp.clone());
                table_rows = table_rows.child(
                    div()
                        .pt(px(10.))
                        .pb(px(2.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(grp.clone()),
                );
            }
            let indented = line.group.is_some();
            let valid = line.has_valid_rate();
            let rate_str = if valid {
                format_money(line.rate, sym, decimals)
            } else {
                "—".to_owned()
            };
            let amt_str = if valid {
                format_money(line.subtotal(), sym, decimals)
            } else {
                "—".to_owned()
            };
            let tax_pct = if line.tax.is_finite() {
                format!("{:.0}%", (line.tax * 100.0).max(0.0))
            } else {
                "—".to_owned()
            };

            let is_added = self.added_line_ids.contains(&line.id);
            let row_el = div()
                .min_h(px(36.))
                .py(px(7.))
                .flex()
                .items_center()
                .gap(px(12.))
                .text_size(px(13.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .when(indented, |d| d.pl(px(14.)))
                        .text_color(p.ink)
                        .truncate()
                        .child(line.name.clone()),
                )
                .child(
                    div()
                        .w(px(58.))
                        .flex_none()
                        .text_right()
                        .text_color(p.text)
                        .child(format_qty(line.qty)),
                )
                .when(!compact, |d| {
                    d.child(
                        div()
                            .w(px(76.))
                            .flex_none()
                            .text_right()
                            .text_color(p.text)
                            .child(rate_str),
                    )
                    .child(
                        div()
                            .w(px(48.))
                            .flex_none()
                            .text_right()
                            .text_color(p.muted)
                            .child(tax_pct),
                    )
                })
                .child(
                    div()
                        .w(px(86.))
                        .flex_none()
                        .text_right()
                        .text_color(p.ink)
                        .child(amt_str),
                );
            table_rows = table_rows.child(AddedLineFold {
                id: pro_eid(&self.id, format!("inv-line-{}", line.id)),
                animate_in: is_added,
                content: row_el.into_any_element(),
            });
        }

        let add_line_row = if !self.issued {
            Some(div().mt(px(6.)).flex().child(pro_btn(
                pro_eid(&self.id, "inv-add-line"),
                tr(cx, "Add line item", "添加明细行"),
                Some(ProIcon::Plus),
                ProBtnStyle::Ghost,
                true,
                cx,
                |this, _, cx| {
                    this.append_addon_line(cx);
                },
            )))
        } else {
            None
        };

        // Totals block right-aligned (rolling numbers when line items or applied credits change)
        let credit_no_lbl = format!("{} ({})", tr(cx, "Credit note", "贷项冲抵"), self.credit_no);
        let cred_prefix = format!("-{sym}");
        let green_ink = p.green_ink;
        let base_id_cred = self.id.clone();

        let totals = div()
            .mt(px(16.))
            .ml_auto()
            .w(px(if compact { 220. } else { 256. }))
            .max_w_full()
            .flex()
            .flex_col()
            .gap(px(5.))
            .text_size(px(13.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_color(p.muted).child(tr(cx, "Subtotal", "小计")))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "inv-roll-sub"),
                        inv_sub,
                        decimals,
                        sym,
                        "",
                        13.,
                        FontWeight::NORMAL,
                        p.ink,
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_color(p.muted).child(tr(cx, "Tax", "税额")))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "inv-roll-tax"),
                        inv_tax,
                        decimals,
                        sym,
                        "",
                        13.,
                        FontWeight::NORMAL,
                        p.ink,
                    )),
            )
            .child(
                div()
                    .mt(px(5.))
                    .pt(px(10.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Total", "合计")),
                    )
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "inv-roll-total"),
                        inv_total,
                        decimals,
                        sym,
                        "",
                        13.,
                        FontWeight::MEDIUM,
                        p.strong,
                    )),
            )
            .child(pro_fold(
                pro_eid(&self.id, "inv-cred-fold"),
                self.issued,
                24.,
                0.,
                move || {
                    div()
                        .pt(px(4.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(12.))
                        .child(div().text_color(green_ink).child(credit_no_lbl))
                        .child(pro_rolling_number(
                            pro_eid(&base_id_cred, "inv-roll-cred"),
                            cred_total,
                            decimals,
                            cred_prefix,
                            "",
                            12.,
                            FontWeight::MEDIUM,
                            green_ink,
                        ))
                        .into_any_element()
                },
            ));

        div()
            .flex()
            .flex_col()
            .child(head)
            .child(parties)
            .child(terms_block)
            .child(table_head)
            .child(table_rows)
            .when_some(add_line_row, |d, row| d.child(row))
            .child(totals)
    }

    fn render_credit_sheet_body(
        &self,
        compact: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let (sym, iso, decimals) = resolve_currency(&self.currency);
        let (cred_sub, cred_tax, cred_total) = self.credit_totals();

        let status_line: SharedString = if self.issued {
            if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!(
                    "已于 {} 签发冲抵 {}",
                    self.invoice_date,
                    format_money(cred_total, sym, decimals)
                )
                .into()
            } else if self.invoice_date.is_empty() {
                format!("{} credited", format_money(cred_total, sym, decimals)).into()
            } else {
                format!(
                    "{} credited on {}",
                    format_money(cred_total, sym, decimals),
                    self.invoice_date
                )
                .into()
            }
        } else if cred_total > 0.000_5 {
            if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("草稿 · 拟冲抵 {}", format_money(cred_total, sym, decimals)).into()
            } else {
                format!(
                    "Draft · {} to credit on {}",
                    format_money(cred_total, sym, decimals),
                    self.invoice_no
                )
                .into()
            }
        } else {
            tr(
                cx,
                "Select at least one line item to credit",
                "请至少勾选一项明细进行贷记冲抵",
            )
            .into()
        };

        let head = div()
            .mb(px(if compact { 22. } else { 28. }))
            .flex()
            .when(compact, |d| d.flex_col().gap(px(10.)))
            .when(!compact, |d| d.items_start().justify_between().gap(px(16.)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .text_size(px(24.))
                                    .line_height(px(32.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Credit note", "贷项通知单")),
                            )
                            .when(self.issued, |d| {
                                d.child(
                                    div()
                                        .h(px(20.))
                                        .px(px(6.))
                                        .rounded(px(4.))
                                        .bg(p.green_soft)
                                        .text_color(p.green_ink)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .flex()
                                        .items_center()
                                        .child(tr(cx, "Issued", "已签发")),
                                )
                            }),
                    )
                    .child(div().mt(px(4.)).child(pro_swap_label(
                        pro_eid(&self.id, "cn-status-lbl"),
                        status_line,
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    ))),
            )
            .child(self.render_seller_brand(&p, cx));

        // Credit note Terms
        let mut terms_dl = div().flex().flex_col().gap(px(3.));
        terms_dl = terms_dl.child(Self::render_terms_row(
            tr(cx, "Credit note", "贷项单号"),
            self.credit_no.clone(),
            compact,
            &p,
        ));
        let inv_jump = activate(
            div()
                .id(pro_eid(&self.id, "cn-for-invoice"))
                .role(Role::Button)
                .tab_index(0)
                .self_start()
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.accent_info)
                .cursor_pointer()
                .child(self.invoice_no.clone()),
            cx,
            |this, _, cx| {
                this.set_view(BillingSheetView::Invoice, cx);
            },
        );
        terms_dl = terms_dl.child(Self::render_terms_row(
            tr(cx, "For invoice", "关联账单"),
            inv_jump,
            compact,
            &p,
        ));
        if !self.invoice_date.is_empty() {
            terms_dl = terms_dl.child(Self::render_terms_row(
                tr(cx, "Credit date", "冲抵日期"),
                self.invoice_date.clone(),
                compact,
                &p,
            ));
        }
        if !self.period.is_empty() {
            terms_dl = terms_dl.child(Self::render_terms_row(
                tr(cx, "Billing period", "计费周期"),
                self.period.clone(),
                compact,
                &p,
            ));
        }

        let terms_block = div()
            .mb(px(if compact { 20. } else { 24. }))
            .flex()
            .flex_col()
            .child(
                div()
                    .mb(px(6.))
                    .text_size(px(10.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "TERMS", "单据条款")),
            )
            .child(terms_dl);

        // Lines to credit tray
        let tray_head = div()
            .h(px(28.))
            .pb(px(6.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_end()
            .gap(px(12.))
            .text_size(px(11.5))
            .text_color(p.muted)
            .child(div().w(px(16.)).flex_none())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(tr(cx, "Line item", "明细项")),
            )
            .when(!compact, |d| {
                d.child(div().w(px(88.)).flex_none().text_right().child(tr(
                    cx,
                    "Billed",
                    "原账单金额",
                )))
            })
            .child(
                div()
                    .w(px(if compact { 116. } else { 148. }))
                    .flex_none()
                    .text_right()
                    .child(tr(cx, "Credit", "冲抵金额")),
            );

        let mut tray_rows = div().pt(px(4.)).flex().flex_col();
        for st in &self.credit_states {
            let Some(line) = self.lines.iter().find(|l| l.id == st.line_id) else {
                continue;
            };
            let check_t = st.check_slide.get(w, cx).clamp(0., 1.);
            let checked = st.checked;
            let lid_chk = st.line_id.clone();
            let lid_minus = st.line_id.clone();
            let lid_plus = st.line_id.clone();
            let step = (st.max_amount * 0.25).max(1.0);
            let cur_amt = st.amount;
            let max_amt = st.max_amount;
            let ed_click = st.editor.clone();
            let issued = self.issued;
            let is_added = self.added_line_ids.contains(&line.id);

            let chk_draw = pro_check_draw(
                pro_eid(&self.id, format!("chk-draw-{}", st.line_id)),
                checked,
                16.,
                true,
                p.accent_info,
            );
            let checkbox = if issued {
                div()
                    .id(pro_eid(&self.id, format!("chk-{}", st.line_id)))
                    .size(px(16.))
                    .flex_none()
                    .child(chk_draw)
                    .into_any_element()
            } else {
                pro_tactile(
                    pro_eid(&self.id, format!("chk-{}", st.line_id)),
                    format!("Credit {}", line.name),
                    5.,
                    transparent_black(),
                    transparent_black(),
                    transparent_black(),
                    transparent_black(),
                    checked,
                    div()
                        .size(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(chk_draw),
                    cx,
                    move |this, w, cx| {
                        this.toggle_credit_line(&lid_chk, Some(w), cx);
                    },
                )
                .flex_none()
                .into_any_element()
            };

            let row_alpha = 0.45 + 0.55 * check_t;

            let amount_box = if issued {
                div()
                    .w(px(if compact { 116. } else { 148. }))
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .text_size(px(13.))
                    .text_color(if checked { p.ink } else { p.muted })
                    .child(format_money(
                        if checked { cur_amt } else { 0.0 },
                        sym,
                        decimals,
                    ))
                    .into_any_element()
            } else {
                div()
                    .w(px(if compact { 116. } else { 148. }))
                    .h(px(28.))
                    .px(px(4.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .border_1()
                    .border_color(p.line)
                    .opacity(0.55 + 0.45 * check_t)
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .child(pro_icon_btn(
                        pro_eid(&self.id, format!("step-dec-{}", st.line_id)),
                        tr(cx, "Decrease credit amount", "减少冲抵金额"),
                        ProIcon::Minus,
                        20.,
                        false,
                        cx,
                        move |this, w, cx| {
                            let next = (cur_amt - step).max(0.0);
                            this.set_credit_amount(&lid_minus, next, Some(w), cx);
                        },
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .px(px(2.))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .cursor_text()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_, _, w, cx| {
                                    if let Some(ed) = &ed_click {
                                        ed.read(cx).focus.clone().focus(w, cx);
                                    }
                                }),
                            )
                            .child(div().text_size(px(12.5)).text_color(p.muted).child(sym))
                            .when_some(st.editor.clone(), |d, ed| {
                                d.child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .h(px(22.))
                                        .overflow_hidden()
                                        .child(ed),
                                )
                            }),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, format!("step-inc-{}", st.line_id)),
                        tr(cx, "Increase credit amount", "增加冲抵金额"),
                        ProIcon::Plus,
                        20.,
                        false,
                        cx,
                        move |this, w, cx| {
                            let next = (cur_amt + step).min(max_amt);
                            this.set_credit_amount(&lid_plus, next, Some(w), cx);
                        },
                    ))
                    .into_any_element()
            };

            let row_el = div()
                .min_h(px(42.))
                .py(px(5.))
                .flex()
                .items_center()
                .gap(px(12.))
                .child(checkbox)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .opacity(row_alpha)
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .truncate()
                        .child(line.name.clone()),
                )
                .when(!compact, |d| {
                    d.child(
                        div()
                            .w(px(88.))
                            .flex_none()
                            .opacity(row_alpha)
                            .text_right()
                            .text_size(px(13.))
                            .text_color(p.text)
                            .child(format_money(line.subtotal(), sym, decimals)),
                    )
                })
                .child(amount_box);

            tray_rows = tray_rows.child(AddedLineFold {
                id: pro_eid(&self.id, format!("cn-line-{}", st.line_id)),
                animate_in: is_added,
                content: row_el.into_any_element(),
            });
        }

        let cn_add_line = if !self.issued {
            Some(div().mt(px(4.)).flex().child(pro_btn(
                pro_eid(&self.id, "cn-add-line"),
                tr(cx, "Add line item", "添加明细行"),
                Some(ProIcon::Plus),
                ProBtnStyle::Ghost,
                true,
                cx,
                |this, _, cx| {
                    this.append_addon_line(cx);
                },
            )))
        } else {
            None
        };

        let tray = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .mb(px(4.))
                    .text_size(px(10.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "LINES TO CREDIT", "冲抵明细行")),
            )
            .child(tray_head)
            .child(tray_rows)
            .when_some(cn_add_line, |d, btn| d.child(btn));

        // Live rolling Credit Note totals
        let adj_prefix = if cred_total > 0.000_5 {
            format!("-{sym}")
        } else {
            sym.to_owned()
        };
        let applied_suffix = format!(" {iso}");

        let totals = div()
            .mt(px(16.))
            .ml_auto()
            .w(px(if compact { 232. } else { 268. }))
            .max_w_full()
            .flex()
            .flex_col()
            .gap(px(5.))
            .text_size(px(13.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_color(p.muted)
                            .child(tr(cx, "Subtotal", "冲抵小计")),
                    )
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "cn-roll-sub"),
                        cred_sub,
                        decimals,
                        sym,
                        "",
                        13.,
                        FontWeight::NORMAL,
                        p.ink,
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_color(p.muted).child(tr(cx, "Tax", "冲抵税额")))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "cn-roll-tax"),
                        cred_tax,
                        decimals,
                        sym,
                        "",
                        13.,
                        FontWeight::NORMAL,
                        p.ink,
                    )),
            )
            .child(
                div()
                    .mt(px(5.))
                    .pt(px(10.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Adjustment total", "冲抵总额")),
                    )
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "cn-roll-total"),
                        cred_total,
                        decimals,
                        adj_prefix.clone(),
                        "",
                        13.,
                        FontWeight::MEDIUM,
                        p.strong,
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(12.))
                    .child(div().text_color(p.muted).child(tr(
                        cx,
                        "Applied to invoice",
                        "应用于原账单",
                    )))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "cn-roll-applied"),
                        cred_total,
                        decimals,
                        adj_prefix,
                        applied_suffix,
                        12.,
                        FontWeight::NORMAL,
                        p.text,
                    )),
            );

        // Issue / Undo footer
        let can_issue = !self.issued && cred_total > 0.000_5;
        let issue_row = if self.issued {
            div()
                .mt(px(22.))
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.))
                .child(pro_btn(
                    pro_eid(&self.id, "undo-credit-btn"),
                    tr(cx, "Edit credit note", "撤回并修改"),
                    Some(ProIcon::Undo),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.undo_credit_note(cx);
                    },
                ))
        } else {
            let (_, _, dec_reset) = resolve_currency(&self.currency);
            let needs_reset = self
                .credit_states
                .iter()
                .any(|s| !s.checked || (s.amount - s.max_amount).abs() > 0.005);
            let reset_btn = pro_btn(
                pro_eid(&self.id, "reset-credit-lines"),
                tr(cx, "Reset full amount", "恢复全额"),
                None,
                ProBtnStyle::Ghost,
                true,
                cx,
                move |this, w, cx| {
                    for st in &mut this.credit_states {
                        st.checked = true;
                        st.amount = st.max_amount;
                        st.check_slide.duration(1., 0.20, cx);
                        if let Some(ed) = &st.editor {
                            let full = format!("{:.dec_reset$}", st.max_amount);
                            ed.update(cx, |e, cx| e.set_value(full, w, cx));
                        }
                    }
                    cx.notify();
                },
            );

            let issue_btn = if can_issue {
                pro_btn(
                    pro_eid(&self.id, "issue-credit-btn"),
                    tr(cx, "Issue credit note", "签发贷项通知单"),
                    Some(ProIcon::Check),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.issue_credit_note(cx);
                    },
                )
                .into_any_element()
            } else {
                div()
                    .id(pro_eid(&self.id, "issue-credit-btn"))
                    .h(px(28.))
                    .px(px(12.))
                    .rounded(px(7.))
                    .bg(p.primary)
                    .text_color(p.on_primary)
                    .opacity(0.40)
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .child(tr(cx, "Issue credit note", "签发贷项通知单"))
                    .into_any_element()
            };

            div()
                .mt(px(22.))
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.))
                .child(pro_popover_presence(
                    pro_eid(&self.id, "reset-credit-pres"),
                    needs_reset,
                    move || reset_btn.into_any_element(),
                ))
                .child(issue_btn)
        };

        div()
            .flex()
            .flex_col()
            .child(head)
            .child(terms_block)
            .child(tray)
            .child(totals)
            .child(issue_row)
    }

    fn render_loading(&self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        if !disabled_motion(cx) {
            w.request_animation_frame();
        }
        let phase = (self.clock.elapsed().as_secs_f32() * 0.75).fract();
        div()
            .max_w(px(680.))
            .w_full()
            .mx_auto()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(pro_skeleton_bar(168., 28., 8., phase, cx))
                    .child(pro_skeleton_bar(148., 28., 8., phase + 0.1, cx)),
            )
            .child(
                div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .p(px(32.))
                    .flex()
                    .flex_col()
                    .gap(px(20.))
                    .child(pro_skeleton_bar(180., 28., 6., phase + 0.15, cx))
                    .child(pro_skeleton_bar(320., 76., 8., phase + 0.25, cx))
                    .child(pro_skeleton_bar(480., 120., 8., phase + 0.35, cx)),
            )
    }
}

impl Render for InvoiceCreditBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_line_editors(w, cx);
        let p = ProPalette::get(cx);
        let measure_cell = self.measured_width.clone();
        let container_w = measure_cell.get().max(320.);
        let compact = container_w <= 560.;
        let ultra_compact = container_w <= 400.;

        let sensor = canvas(
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
        .left_0()
        .right_0()
        .top_0()
        .h(px(0.));

        if self.loading {
            return div()
                .id(self.id.clone())
                .w_full()
                .py(px(8.))
                .relative()
                .child(sensor)
                .child(self.render_loading(w, cx))
                .into_any_element();
        }

        if self.empty || self.lines.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .relative()
                .rounded(px(12.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(sensor)
                .child(pro_empty_state(
                    ProIcon::ClipboardDoc,
                    tr(cx, "No invoice lines to display", "暂无可显示的账单明细"),
                    tr(
                        cx,
                        "Provide billed line items to preview stacked invoice and credit note sheets.",
                        "传入账单明细行即可预览叠层账单与贷项通知单据。",
                    ),
                    Some((
                        pro_eid(&self.id, "restore-invoice"),
                        tr(cx, "Load sample invoice", "载入示例账单").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            if this.lines.is_empty() {
                                this.lines = default_invoice_lines();
                                this.credit_states = Self::build_credit_states(&this.lines);
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        // Top bar: Segmented switch (Invoice | Credit note) + actions (PDF | Credit invoice)
        let seg_sensor = pro_selection_sensor();
        let seg_thumb: Hsla = if p.dark {
            rgb(0x2c2c2e).into()
        } else {
            rgb(0xffffff).into()
        };
        let mut seg = div()
            .id(pro_eid(&self.id, "doc-switch"))
            .role(Role::TabList)
            .aria_label(tr(cx, "Document", "单据视图"))
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .relative()
            .flex()
            .items_center()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "left" | "home" => {
                        this.set_view(BillingSheetView::Invoice, cx);
                        cx.stop_propagation();
                    }
                    "right" | "end" => {
                        this.set_view(BillingSheetView::CreditNote, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "doc-switch-pill"),
                self.view.key(),
                seg_sensor.clone(),
                6.,
                seg_thumb,
                Some(p.line),
                true,
            ));

        for (v_kind, lbl) in [
            (BillingSheetView::Invoice, tr(cx, "Invoice", "账单发票")),
            (
                BillingSheetView::CreditNote,
                tr(cx, "Credit note", "贷项通知单"),
            ),
        ] {
            let active = self.view == v_kind;
            seg = seg.child(
                pro_tactile(
                    pro_eid(&self.id, format!("doc-tab-{}", v_kind.key())),
                    lbl,
                    6.,
                    transparent_black(),
                    if active { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    active,
                    div()
                        .h(px(28.))
                        .px(px(if ultra_compact { 9. } else { 12. }))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if active { p.ink } else { p.text })
                        .child(lbl),
                    cx,
                    move |this, _, cx| {
                        this.set_view(v_kind, cx);
                    },
                )
                .relative()
                .flex_none()
                .when(active, |d| d.child(pro_mark_selected(&seg_sensor))),
            );
        }

        let cur_view = self.view;
        let pdf_btn = pro_btn(
            pro_eid(&self.id, "pdf-btn"),
            "PDF",
            Some(ProIcon::Download),
            ProBtnStyle::Secondary,
            true,
            cx,
            move |this, _, cx| {
                this.toast = Some((
                    tr(cx, "Prepared PDF document export", "已准备 PDF 单据导出").into(),
                    Instant::now(),
                ));
                cx.emit(InvoiceCreditEvent::PdfRequested(cur_view));
                cx.notify();
            },
        );

        let (flip_lbl, flip_icon, flip_next) = match self.view {
            BillingSheetView::Invoice => (
                if self.issued {
                    tr(cx, "View credit note", "查看贷项单")
                } else {
                    tr(cx, "Credit invoice", "开具贷项冲抵")
                },
                ProIcon::Undo,
                BillingSheetView::CreditNote,
            ),
            BillingSheetView::CreditNote => (
                tr(cx, "View invoice", "返回原账单"),
                ProIcon::ArrowLeft,
                BillingSheetView::Invoice,
            ),
        };
        let flip_btn = pro_btn(
            pro_eid(&self.id, "bar-credit-invoice"),
            flip_lbl,
            Some(flip_icon),
            ProBtnStyle::Secondary,
            true,
            cx,
            move |this, _, cx| {
                this.set_view(flip_next, cx);
            },
        );

        let top_bar = div()
            .mb(px(12.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(seg)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .when(!compact, |d| d.child(pdf_btn))
                    .child(flip_btn),
            );

        // Stacked-paper spring geometry:
        // s = 0.0 -> Invoice in front (depth=0), Credit Note peeking 12px above in back (depth=1)
        // s = 1.0 -> Credit Note in front (depth=0), Invoice peeking 12px above in back (depth=1)
        let s = self.sheet_slide.get(w, cx).clamp(0., 1.);
        let back_surface = pro_mix_color(p.card, p.subtle, 0.55);
        let pad_x = if ultra_compact {
            16.
        } else if compact {
            20.
        } else {
            34.
        };
        let pad_y = if compact { 22. } else { 30. };

        let inv_depth = s;
        let cn_depth = 1. - s;

        let inv_body = self.render_invoice_sheet_body(compact, cx);
        let cn_body = self.render_credit_sheet_body(compact, w, cx);

        let build_front_sheet = |id_sfx: &'static str, depth: f32, body: Div| {
            let inset_x = 13. * depth;
            let top_y = 12. * (1. - depth);
            let content_alpha = (1. - depth * 1.65).clamp(0., 1.);
            div()
                .id(pro_eid(&self.id, id_sfx))
                .role(Role::TabPanel)
                .mt(px(top_y))
                .mx(px(inset_x))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .px(px(pad_x))
                .py(px(pad_y))
                .child(div().opacity(content_alpha).child(body))
        };

        let back_peek_target = if s < 0.5 {
            BillingSheetView::CreditNote
        } else {
            BillingSheetView::Invoice
        };
        let back_depth = if s < 0.5 { cn_depth } else { inv_depth };
        let back_inset = 13. * back_depth;
        let back_top = 12. * (1. - back_depth);
        let back_aria = match back_peek_target {
            BillingSheetView::CreditNote => {
                tr(cx, "Bring Credit note to front", "切换至贷项通知单")
            }
            BillingSheetView::Invoice => tr(cx, "Bring Invoice to front", "切换至原账单"),
        };

        let back_sheet_peek = BackSheetPeek {
            id: pro_eid(&self.id, "back-sheet-peek"),
            aria: back_aria.into(),
            top: back_top,
            inset: back_inset,
            rest_bg: back_surface,
            hover_bg: p.card,
            border: p.line_strong,
            on_click: Rc::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.set_view(back_peek_target, cx);
            })),
        };

        let front_sheet = if s < 0.5 {
            build_front_sheet("sheet-invoice", inv_depth, inv_body)
        } else {
            build_front_sheet("sheet-credit", cn_depth, cn_body)
        };

        let stack = div()
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .child(back_sheet_peek)
            .child(front_sheet);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(tr(cx, "Invoice and credit note", "账单与贷项通知单"))
            .w_full()
            .px(px(if compact { 16. } else { 36. }))
            .py(px(if compact { 20. } else { 32. }))
            .relative()
            .flex()
            .flex_col()
            .child(sensor)
            .child(
                div()
                    .w_full()
                    .max_w(px(680.))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .child(top_bar)
                    .child(stack),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BillingSheetView, InvoiceCreditBlock, InvoiceLine, default_invoice_lines, format_money,
    };
    use gpui::TestAppContext;

    #[test]
    fn default_invoice_lines_match_reference_subtotal_tax_and_total() {
        let lines = default_invoice_lines();
        assert_eq!(lines.len(), 3);
        let subtotal: f64 = lines.iter().map(InvoiceLine::subtotal).sum();
        let tax: f64 = lines.iter().map(|l| l.tax_for(l.subtotal())).sum();
        let total = subtotal + tax;
        assert!((subtotal - 9_466.40).abs() < 1e-6);
        assert!((tax - 1_893.28).abs() < 1e-6);
        assert!((total - 11_359.68).abs() < 1e-6);
        assert_eq!(format_money(total, "$", 2), "$11,359.68");
    }

    #[test]
    fn non_finite_and_discount_rates_follow_spec() {
        let nan_line = InvoiceLine::new("bad", "Pending meter", 1.0, f64::NAN, 0.2);
        assert!(!nan_line.has_valid_rate());
        assert_eq!(nan_line.subtotal(), 0.0);
        assert!(!nan_line.is_creditable());

        let discount = InvoiceLine::new("disc", "Annual commitment discount", 1.0, -200.0, 0.2);
        assert_eq!(discount.subtotal(), -200.0);
        assert!(!discount.is_creditable());
    }

    #[test]
    fn adjusting_credit_lines_clamps_amounts_and_issues_with_undo() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (block, w) =
            app.add_window_view(|_, cx| InvoiceCreditBlock::new("test-invoice-credit", cx));

        w.update(|_, cx| {
            block.update(cx, |ivc, cx| {
                assert_eq!(ivc.view(), BillingSheetView::Invoice);
                let (_, _, init_total) = ivc.credit_totals();
                assert!((init_total - 11_359.68).abs() < 1e-4);

                // Clamp above line subtotal
                ivc.set_credit_amount("plan", 99_999.0, None, cx);
                let (sub_clamped, _, _) = ivc.credit_totals();
                assert!((sub_clamped - 9_466.40).abs() < 1e-4);

                // Uncheck metered API line ($7,736.40 pre-tax)
                assert!(ivc.toggle_credit_line("api", None, cx));
                let (sub_partial, tax_partial, tot_partial) = ivc.credit_totals();
                assert!((sub_partial - 1_730.00).abs() < 1e-4);
                assert!((tax_partial - 346.00).abs() < 1e-4);
                assert!((tot_partial - 2_076.00).abs() < 1e-4);

                // Issue & undo
                assert!(ivc.issue_credit_note(cx));
                assert!(ivc.is_issued());
                assert!(!ivc.toggle_credit_line("plan", None, cx));

                assert!(ivc.undo_credit_note(cx));
                assert!(!ivc.is_issued());
            });
        });
    }
}
