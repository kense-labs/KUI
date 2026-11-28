//! Authentic interactive K Language IDE (`K for Visual Studio Code` + `k lsp` compiler server).

use super::{is_dark, is_zh, tr, traffic_lights};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum BottomTab {
    Problems,
    LspOutput,
    Terminal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CodeKind {
    Comment,
    Decl,
    Proof,
    Body,
    ErrorLine,
}

pub(super) struct KlangIdeApp {
    pub(super) file_idx: usize,
    pub(super) selected_line: usize,
    pub(super) inlay_hints: bool,
    pub(super) inject_e0312: bool,
    pub(super) bottom_tab: BottomTab,
    pub(super) run_count: usize,
    pub(super) term_lines: Vec<SharedString>,
    pub(super) lsp_lines: Vec<SharedString>,
}

struct KSourceFile {
    path: &'static str,
    name: &'static str,
    outline: [&'static str; 4],
    lines: [(&'static str, Option<&'static str>, CodeKind); 13],
    jit_cmd: &'static str,
    jit_out: &'static str,
}

const FILES: [KSourceFile; 3] = [
    KSourceFile {
        path: "bench/klang/graph_csr.k",
        name: "graph_csr.k",
        outline: [
            "type Vertex = u32 in dist",
            "type EdgeEnd = u32 upto out",
            "checkfree fn bfs(...) -> i64",
            "fn main()",
        ],
        lines: [
            (
                "// Real CSR breadth-first search from kense-labs/klang",
                None,
                CodeKind::Comment,
            ),
            (
                "var dist: i32[>..] = [];",
                Some("/* data-stack grow-only */"),
                CodeKind::Decl,
            ),
            (
                "type Vertex = u32 in dist;",
                Some("/* proven: 0 <= v < dist.len */"),
                CodeKind::Proof,
            ),
            ("var out: Vertex[>..] = [];", None, CodeKind::Decl),
            (
                "type EdgeEnd = u32 upto out;",
                Some("/* proven: 0 <= e <= out.len */"),
                CodeKind::Proof,
            ),
            (
                "checkfree fn bfs(src: Vertex, start: EdgeEnd[>..]&, q: Vertex[>..<]&) -> i64 {",
                None,
                CodeKind::Decl,
            ),
            (
                "    assert(start.len > dist.len);",
                Some("/* discharges start[u + 1] */"),
                CodeKind::Proof,
            ),
            (
                "    dist[src] = 0; q.push(src);",
                Some("/* 0 bounds checks */"),
                CodeKind::Body,
            ),
            (
                "    for k in start[u]..start[u as i64 + 1] {",
                None,
                CodeKind::Body,
            ),
            (
                "        let w = out[k];",
                Some("/* : Vertex (in-bounds) */"),
                CodeKind::Body,
            ),
            (
                "        if dist[w] < 0 { dist[w] = d + 1; q.push(w); }",
                None,
                CodeKind::Body,
            ),
            ("    }", None, CodeKind::Body),
            ("}", None, CodeKind::Decl),
        ],
        jit_cmd: "$ k run --jit bench/klang/graph_csr.k",
        jit_out: "JIT (TinyCC) compiled in 14 ms -> 9000000 edges · BFS checksum 49810422 (54.2 ms, 1.00x C++)",
    },
    KSourceFile {
        path: "samples/02_memory.k",
        name: "02_memory.k",
        outline: [
            "struct Item { id, weight }",
            "struct Row { label, cells }",
            "fn reset(buf: u8[>..]&)",
            "fn main()",
        ],
        lines: [
            (
                "// Data-stack arena: growth is a pointer bump; scope exit is O(1) free.",
                None,
                CodeKind::Comment,
            ),
            (
                "struct Item { id: i32, weight: f32 }",
                Some("/* 8 bytes flat */"),
                CodeKind::Decl,
            ),
            ("fn main() {", None, CodeKind::Decl),
            (
                "    var items: Item[>..] = [];",
                Some("/* frame #1 on data stack */"),
                CodeKind::Body,
            ),
            (
                "    let first: Item& = items.push(Item { 1, 0.5 });",
                Some("/* pinned ref across pushes */"),
                CodeKind::Proof,
            ),
            (
                "    for i in 2..100001 { items.push(Item { i as i32, 0.5 }); }",
                None,
                CodeKind::Body,
            ),
            (
                "    first.weight = 99.5;",
                Some("/* valid after 100k pushes! */"),
                CodeKind::Body,
            ),
            ("    for round in 3 {", None, CodeKind::Body),
            (
                "        var scratch: u8[>..] = [];",
                Some("/* freed by 1 sp reset at '}' */"),
                CodeKind::Proof,
            ),
            (
                "        for i in 10000 { scratch.push((i % 256) as! u8); }",
                None,
                CodeKind::Body,
            ),
            ("    }", None, CodeKind::Body),
            (
                "    print(items.len, \" items, first=\", items[0].weight);",
                None,
                CodeKind::Body,
            ),
            ("}", None, CodeKind::Decl),
        ],
        jit_cmd: "$ k run --jit samples/02_memory.k",
        jit_out: "100000 items, first=99.5 | 3x 10000B scratch freed in 0 ns (0 malloc / 0 free)",
    },
    KSourceFile {
        path: "bench/klang/respond.k",
        name: "respond.k",
        outline: [
            "struct Item { sku, qty, price }",
            "struct Resp { id, user, items }",
            "fn render(out: u8[>..]&, r: Resp&)",
            "fn main()",
        ],
        lines: [
            (
                "// Contiguous wire-packed DTO: strings & item slices inline behind varint.",
                None,
                CodeKind::Comment,
            ),
            (
                "struct Item { sku: u8[varint], qty: varint, price: varint }",
                Some("/* 0 heap pointers */"),
                CodeKind::Decl,
            ),
            (
                "struct Resp { id: varint, user: u8[varint], items: Item[varint] }",
                Some("/* 1 contiguous block */"),
                CodeKind::Proof,
            ),
            ("fn render(out: u8[>..]&, r: Resp&) {", None, CodeKind::Decl),
            ("    out.append(\"{\\\"id\\\":\");", None, CodeKind::Body),
            (
                "    emit_int(out, r.id);",
                Some("/* tmp: u8[..24] inline */"),
                CodeKind::Body,
            ),
            ("    out.append(\",\\\"user\\\":\");", None, CodeKind::Body),
            ("    emit_str(out, r.user[..]);", None, CodeKind::Body),
            (
                "    for it in r.items {",
                Some("/* sequential cache scan */"),
                CodeKind::Proof,
            ),
            ("        emit_item(out, it);", None, CodeKind::Body),
            ("    }", None, CodeKind::Body),
            ("    out.append(\"]}\");", None, CodeKind::Body),
            ("}", None, CodeKind::Decl),
        ],
        jit_cmd: "$ k run --jit bench/klang/respond.k",
        jit_out: "400,000 JSON DTO responses built & freed by sp reset -> 41.8 ms (2.6x faster than heap DTO)",
    },
];

impl KlangIdeApp {
    pub(super) fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            file_idx: 0,
            selected_line: 5,
            inlay_hints: true,
            inject_e0312: false,
            bottom_tab: BottomTab::Terminal,
            run_count: 1,
            term_lines: vec![
                "$ k lsp --probe".into(),
                "k lsp server v0.4 ready · entryFile = bench/klang/graph_csr.k".into(),
                "$ k check --diagnostics json bench/klang/graph_csr.k".into(),
                "✓ 0 errors, 0 warnings · checkfree fn bfs verified (0 bounds checks)".into(),
            ],
            lsp_lines: vec![
                "[k lsp] analyzed 3 workspace files in 3.8 ms".into(),
                "[k lsp] checkfree proof: out[k] bounded by EdgeEnd (u32 upto out)".into(),
                "[k lsp] checkfree proof: dist[w] bounded by Vertex (u32 in dist)".into(),
                "[k lsp] data-stack escape analysis: 0 escaping references".into(),
            ],
        }
    }

    fn run_jit(&mut self) {
        let f = &FILES[self.file_idx];
        self.run_count += 1;
        if self.inject_e0312 {
            self.bottom_tab = BottomTab::Problems;
            return;
        }
        self.bottom_tab = BottomTab::Terminal;
        self.term_lines.push(f.jit_cmd.into());
        self.term_lines.push(f.jit_out.into());
    }

    fn emit_c_or_wasi(&mut self) {
        let f = &FILES[self.file_idx];
        if self.inject_e0312 {
            self.bottom_tab = BottomTab::Problems;
            return;
        }
        self.bottom_tab = BottomTab::Terminal;
        self.term_lines
            .push(format!("$ k -o build/{}.c {}", f.name, f.path).into());
        self.term_lines
            .push("✓ Generated freestanding C99 + WASI binary (0 runtime malloc symbols)".into());
    }

    fn render_explorer(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let act_bg = if dark { rgb(0x141519) } else { rgb(0xeef1f6) };
        let side_bg = if dark { rgb(0x18191e) } else { rgb(0xf6f8fb) };
        let border_c = if dark { rgb(0x262830) } else { rgb(0xdce1ea) };
        let active_bg = if dark { rgb(0x262934) } else { rgb(0xdbeafe) };
        let ink = if dark { rgb(0xf4f4f6) } else { rgb(0x0f172a) };
        let sub = if dark { rgb(0xa1a1aa) } else { rgb(0x475569) };
        let muted = if dark { rgb(0x71717a) } else { rgb(0x94a3b8) };

        let mut file_rows = div().flex().flex_col().gap(px(2.));
        for (idx, f) in FILES.iter().enumerate() {
            let active = self.file_idx == idx;
            file_rows = file_rows.child(
                div()
                    .id(("kl-exp-file", idx))
                    .cursor_pointer()
                    .px(px(8.))
                    .py(px(5.))
                    .rounded(px(5.))
                    .bg(if active { active_bg } else { rgba(0x00000000) })
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(11.))
                    .text_color(if active { ink } else { sub })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.file_idx = idx;
                        v.selected_line = 2;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .font_family("Fira Code")
                            .font_weight(FontWeight::BOLD)
                            .text_size(px(10.))
                            .text_color(rgb(0xf43f5e))
                            .child("K"),
                    )
                    .child(div().font_family("Fira Code").child(f.name)),
            );
        }

        let cur = &FILES[self.file_idx];
        let mut outline_rows = div().flex().flex_col().gap(px(3.));
        for (o_idx, sym) in cur.outline.iter().enumerate() {
            outline_rows = outline_rows.child(
                div()
                    .id(("kl-outline-sym", o_idx))
                    .cursor_pointer()
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(px(4.))
                    .font_family("Fira Code")
                    .text_size(px(10.))
                    .text_color(sub)
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.selected_line = (o_idx * 2 + 2).min(11);
                        cx.notify();
                    }))
                    .child(format!(
                        "{} {sym}",
                        if sym.starts_with("fn") || sym.starts_with("checkfree") {
                            "f"
                        } else {
                            "t"
                        }
                    )),
            );
        }

        div()
            .flex()
            .h_full()
            .child(
                // 38px VS Code Activity Bar
                div()
                    .w(px(38.))
                    .flex_none()
                    .h_full()
                    .bg(act_bg)
                    .border_r_1()
                    .border_color(border_c)
                    .py(px(10.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(12.))
                            .font_family("Fira Code")
                            .text_size(px(11.))
                            .child(
                                div()
                                    .w(px(26.))
                                    .h(px(26.))
                                    .rounded(px(6.))
                                    .bg(rgb(0xe11d48))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xffffff))
                                    .child("K"),
                            )
                            .child(div().text_color(ink).child("[]"))
                            .child(div().text_color(muted).child("?"))
                            .child(div().text_color(muted).child("+-")),
                    )
                    .child(
                        div()
                            .font_family("Fira Code")
                            .text_size(px(9.5))
                            .text_color(muted)
                            .child("lsp"),
                    ),
            )
            .child(
                // 184px Explorer + Outline
                div()
                    .w(px(184.))
                    .flex_none()
                    .h_full()
                    .bg(side_bg)
                    .border_r_1()
                    .border_color(border_c)
                    .p(px(8.))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .px(px(4.))
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(tr(zh, "资源管理器 · KLANG", "EXPLORER · KLANG")),
                            )
                            .child(file_rows)
                            .child(div().my(px(4.)).h(px(1.)).bg(border_c))
                            .child(
                                div()
                                    .px(px(4.))
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(tr(zh, "大纲 · K LSP", "OUTLINE · K LSP")),
                            )
                            .child(outline_rows),
                    )
                    .child(
                        div()
                            .p(px(7.))
                            .rounded(px(6.))
                            .bg(if dark { rgb(0x121318) } else { rgb(0xeef2f7) })
                            .border_1()
                            .border_color(border_c)
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .font_family("Fira Code")
                            .text_size(px(9.5))
                            .child(
                                div()
                                    .text_color(rgb(0x22c55e))
                                    .child("● klang-language.vsix"),
                            )
                            .child(div().text_color(muted).child("TinyCC JIT · C99 · WASI")),
                    ),
            )
    }

    fn render_editor_and_panel(
        &self,
        zh: bool,
        dark: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let ed_bg = if dark { rgb(0x0f1014) } else { rgb(0xffffff) };
        let tab_strip_bg = if dark { rgb(0x16171d) } else { rgb(0xf1f5f9) };
        let panel_bg = if dark { rgb(0x121318) } else { rgb(0xf8fafc) };
        let border_c = if dark { rgb(0x242630) } else { rgb(0xdce1ea) };
        let ink = if dark { rgb(0xf4f4f6) } else { rgb(0x0f172a) };
        let sub = if dark { rgb(0xa1a1aa) } else { rgb(0x475569) };
        let muted = if dark { rgb(0x6b7280) } else { rgb(0x94a3b8) };

        let cur = &FILES[self.file_idx];

        // Open editor tabs
        let mut tabs = div().flex().items_center();
        for (idx, f) in FILES.iter().enumerate() {
            let active = self.file_idx == idx;
            tabs = tabs.child(
                div()
                    .id(("kl-ed-tab", idx))
                    .cursor_pointer()
                    .h(px(32.))
                    .px(px(12.))
                    .bg(if active { ed_bg } else { tab_strip_bg })
                    .border_r_1()
                    .border_color(border_c)
                    .border_t_2()
                    .border_color(if active {
                        rgb(0xe11d48)
                    } else {
                        rgba(0x00000000)
                    })
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .font_family("Fira Code")
                    .text_size(px(11.))
                    .text_color(if active { ink } else { muted })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.file_idx = idx;
                        cx.notify();
                    }))
                    .child(div().text_color(rgb(0xf43f5e)).child("K"))
                    .child(f.name),
            );
        }

        // Interactive `k lsp` Code Lenses bar (exact vscode/README.md!)
        let lens_btn =
            |id: &'static str, label: &'static str, accent: Option<Rgba>| -> Stateful<Div> {
                div()
                    .id(id)
                    .debug_selector(move || id.into())
                    .cursor_pointer()
                    .px(px(7.))
                    .py(px(2.))
                    .rounded(px(4.))
                    .bg(accent.unwrap_or(if dark { rgb(0x1b1d26) } else { rgb(0xeef2f7) }))
                    .font_family("Fira Code")
                    .text_size(px(10.))
                    .text_color(if accent.is_some() {
                        rgb(0xffffff)
                    } else if dark {
                        rgb(0x93c5fd)
                    } else {
                        rgb(0x1d4ed8)
                    })
                    .child(label)
            };

        let mut code_rows = div()
            .flex_1()
            .min_h_0()
            .id("kl-code-scroll")
            .overflow_y_scroll()
            .py(px(6.))
            .flex()
            .flex_col();

        for (ln_idx, (code, hint, kind)) in cur.lines.iter().enumerate() {
            let selected = self.selected_line == ln_idx;
            let is_err_line = self.inject_e0312 && ln_idx == 8;
            let row_bg = if is_err_line {
                rgba(0xe11d482b)
            } else if selected {
                if dark { rgb(0x191c26) } else { rgb(0xeff6ff) }
            } else {
                rgba(0x00000000)
            };

            let code_color = match if is_err_line {
                CodeKind::ErrorLine
            } else {
                *kind
            } {
                CodeKind::Comment => {
                    if dark {
                        rgb(0x6b7280)
                    } else {
                        rgb(0x64748b)
                    }
                }
                CodeKind::Decl => {
                    if dark {
                        rgb(0xfda4af)
                    } else {
                        rgb(0xbe123c)
                    }
                }
                CodeKind::Proof => {
                    if dark {
                        rgb(0x7dd3fc)
                    } else {
                        rgb(0x0369a1)
                    }
                }
                CodeKind::Body => ink,
                CodeKind::ErrorLine => rgb(0xfb7185),
            };

            let display_code = if is_err_line {
                "        return &scratch[0];              // <-- E0312: escapes data-stack frame!"
            } else {
                *code
            };

            code_rows = code_rows.child(
                div()
                    .id(("kl-code-ln", ln_idx))
                    .cursor_pointer()
                    .px(px(12.))
                    .py(px(2.))
                    .bg(row_bg)
                    .flex()
                    .items_center()
                    .justify_between()
                    .font_family("Fira Code")
                    .text_size(px(11.))
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.selected_line = ln_idx;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .w(px(20.))
                                    .text_color(muted)
                                    .child(format!("{:>2}", ln_idx + 1)),
                            )
                            .child(div().text_color(code_color).child(display_code)),
                    )
                    .when(self.inlay_hints && !is_err_line, |d| {
                        d.when_some(*hint, |d, h| {
                            d.child(
                                div()
                                    .px(px(5.))
                                    .rounded(px(3.))
                                    .bg(if dark { rgb(0x171a22) } else { rgb(0xf1f5f9) })
                                    .text_size(px(9.5))
                                    .text_color(rgb(0x34d399))
                                    .child(h),
                            )
                        })
                    }),
            );
        }

        // Bottom Panel (PROBLEMS | K OUTPUT | TERMINAL)
        let p_tab = |id: &'static str, tab: BottomTab, label: SharedString| {
            let active = self.bottom_tab == tab;
            div()
                .id(id)
                .cursor_pointer()
                .px(px(8.))
                .py(px(4.))
                .border_b_2()
                .border_color(if active {
                    rgb(0xe11d48)
                } else {
                    rgba(0x00000000)
                })
                .text_size(px(10.5))
                .font_weight(if active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if active { ink } else { muted })
                .on_click(cx.listener(move |v, _, _, cx| {
                    v.bottom_tab = tab;
                    cx.notify();
                }))
                .child(label)
        };

        let prob_label: SharedString = if self.inject_e0312 {
            tr(zh, "问题 PROBLEMS (1)", "PROBLEMS (1)").into()
        } else {
            tr(zh, "问题 PROBLEMS (0)", "PROBLEMS (0)").into()
        };

        let panel_body: AnyElement = match self.bottom_tab {
            BottomTab::Problems => {
                if self.inject_e0312 {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .font_family("Fira Code")
                        .text_size(px(10.5))
                        .child(
                            div()
                                .text_color(rgb(0xfb7185))
                                .child(format!(
                                    "× {}:9:16  error[E0312]: reference into local data-stack frame `scratch` cannot escape scope",
                                    cur.path
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .text_color(sub)
                                        .child("  note: `scratch` resets stack pointer in O(1) at line 11"),
                                )
                                .child(
                                    div()
                                        .id("kl-quickfix-e0312")
                                        .debug_selector(|| "kl-quickfix-e0312".into())
                                        .cursor_pointer()
                                        .px(px(7.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .bg(rgb(0x22c55e))
                                        .text_color(rgb(0x052e16))
                                        .font_weight(FontWeight::BOLD)
                                        .on_click(cx.listener(|v, _, _, cx| {
                                            v.inject_e0312 = false;
                                            cx.notify();
                                        }))
                                        .child(tr(zh, "一键修复 Quick Fix", "Quick Fix E0312")),
                                ),
                        )
                        .into_any_element()
                } else {
                    div()
                        .font_family("Fira Code")
                        .text_size(px(10.5))
                        .text_color(rgb(0x22c55e))
                        .child(tr(
                            zh,
                            "✓ k lsp 实时检查通过：0 个错误，0 个警告 · 所有数据栈引用均未越出作用域且无越界分支。",
                            "✓ No problems detected in workspace: 0 escaping references, 0 bounds checks in `checkfree`.",
                        ))
                        .into_any_element()
                }
            }
            BottomTab::LspOutput => {
                let mut col = div()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .font_family("Fira Code")
                    .text_size(px(10.));
                for l in &self.lsp_lines {
                    col = col.child(div().text_color(sub).child(l.clone()));
                }
                col.into_any_element()
            }
            BottomTab::Terminal => {
                let mut col = div()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .font_family("Fira Code")
                    .text_size(px(10.));
                let start = self.term_lines.len().saturating_sub(5);
                for l in &self.term_lines[start..] {
                    let c = if l.starts_with('$') {
                        rgb(0x93c5fd)
                    } else if l.starts_with('✓') || l.contains("1.00x") || l.contains("0 malloc")
                    {
                        rgb(0x34d399)
                    } else {
                        ink
                    };
                    col = col.child(div().text_color(c).child(l.clone()));
                }
                col.into_any_element()
            }
        };

        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(ed_bg)
            .flex()
            .flex_col()
            // Editor tab bar
            .child(
                div()
                    .h(px(32.))
                    .bg(tab_strip_bg)
                    .border_b_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(tabs)
                    .child(
                        div()
                            .px(px(10.))
                            .font_family("Fira Code")
                            .text_size(px(10.))
                            .text_color(muted)
                            .child(cur.path),
                    ),
            )
            // Code Lens action bar (`k lsp` lenses)
            .child(
                div()
                    .px(px(12.))
                    .py(px(6.))
                    .bg(if dark { rgb(0x13151c) } else { rgb(0xf8fafc) })
                    .border_b_1()
                    .border_color(border_c)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                lens_btn(
                                    "kl-lens-jit",
                                    tr(zh, "▶ Run with JIT", "▶ Run with JIT"),
                                    Some(rgb(0xe11d48)),
                                )
                                .on_click(cx.listener(
                                    |v, _, _, cx| {
                                        v.run_jit();
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                lens_btn(
                                    "kl-lens-wasi",
                                    tr(zh, "生成 C99 / WASI", "Emit C99 / WASI"),
                                    None,
                                )
                                .on_click(cx.listener(
                                    |v, _, _, cx| {
                                        v.emit_c_or_wasi();
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                lens_btn(
                                    "kl-lens-e0312",
                                    if self.inject_e0312 {
                                        tr(zh, "✓ 修复 E0312 越出", "✓ Fix E0312 escape")
                                    } else {
                                        tr(zh, "! 注入 E0312 越出测试", "! Test E0312 escape")
                                    },
                                    if self.inject_e0312 {
                                        Some(rgb(0x16a34a))
                                    } else {
                                        None
                                    },
                                )
                                .on_click(cx.listener(
                                    |v, _, _, cx| {
                                        v.inject_e0312 = !v.inject_e0312;
                                        v.bottom_tab = BottomTab::Problems;
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(
                        div()
                            .id("kl-lens-hints")
                            .cursor_pointer()
                            .px(px(7.))
                            .py(px(2.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(border_c)
                            .font_family("Fira Code")
                            .text_size(px(9.5))
                            .text_color(if self.inlay_hints {
                                rgb(0x34d399)
                            } else {
                                muted
                            })
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.inlay_hints = !v.inlay_hints;
                                cx.notify();
                            }))
                            .child(if self.inlay_hints {
                                tr(zh, "Inlay 证明提示: 开", "Inlay proofs: ON")
                            } else {
                                tr(zh, "Inlay 证明提示: 关", "Inlay proofs: OFF")
                            }),
                    ),
            )
            // Code editor body
            .child(code_rows)
            // Bottom Diagnostics / Output / Terminal Panel
            .child(
                div()
                    .h(px(142.))
                    .flex_none()
                    .bg(panel_bg)
                    .border_t_1()
                    .border_color(border_c)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px(px(12.))
                            .border_b_1()
                            .border_color(border_c)
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(p_tab("kl-ptab-prob", BottomTab::Problems, prob_label))
                            .child(p_tab(
                                "kl-ptab-lsp",
                                BottomTab::LspOutput,
                                "K OUTPUT (k lsp)".into(),
                            ))
                            .child(p_tab(
                                "kl-ptab-term",
                                BottomTab::Terminal,
                                tr(zh, "终端 TERMINAL", "TERMINAL").into(),
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .id("kl-panel-scroll")
                            .overflow_y_scroll()
                            .px(px(12.))
                            .py(px(8.))
                            .child(panel_body),
                    ),
            )
    }
}

impl Render for KlangIdeApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let zh = is_zh(cx);
        let dark = is_dark(cx);
        let title_bg = if dark { rgb(0x16171d) } else { rgb(0xeef1f6) };
        let border_c = if dark { rgb(0x262830) } else { rgb(0xd4dae3) };
        let ink = if dark { rgb(0xf4f4f6) } else { rgb(0x0f172a) };
        let muted = if dark { rgb(0x8e929f) } else { rgb(0x64748b) };
        let status_bg = if self.inject_e0312 {
            rgb(0xbe123c)
        } else {
            rgb(0x0066b8)
        };

        let cur = &FILES[self.file_idx];

        div()
            .size_full()
            .rounded(px(10.))
            .border_1()
            .border_color(border_c)
            .overflow_hidden()
            .flex()
            .flex_col()
            // Native VS Code macOS titlebar
            .child(
                div()
                    .h(px(30.))
                    .px(px(12.))
                    .bg(title_bg)
                    .border_b_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(traffic_lights())
                    .child(
                        div()
                            .font_family("Fira Code")
                            .text_size(px(11.))
                            .text_color(ink)
                            .child(format!("{} — klang — Visual Studio Code", cur.name)),
                    )
                    .child(
                        div()
                            .font_family("Fira Code")
                            .text_size(px(10.))
                            .text_color(muted)
                            .child("k lsp v0.4"),
                    ),
            )
            // Main workspace row
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.render_explorer(zh, dark, cx))
                    .child(self.render_editor_and_panel(zh, dark, cx)),
            )
            // VS Code Status Bar (exact vscode/README.md `✓ K` + `▶ Run`!)
            .child(
                div()
                    .h(px(23.))
                    .px(px(10.))
                    .bg(status_bg)
                    .flex()
                    .items_center()
                    .justify_between()
                    .font_family("Fira Code")
                    .text_size(px(10.))
                    .text_color(rgb(0xffffff))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child("main*")
                            .child(
                                div()
                                    .id("kl-status-check")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|v, _, _, cx| {
                                        v.bottom_tab = BottomTab::Problems;
                                        cx.notify();
                                    }))
                                    .child(if self.inject_e0312 {
                                        "× K: 1 error (E0312)"
                                    } else {
                                        "✓ K · 0 errors"
                                    }),
                            )
                            .child(
                                div()
                                    .id("kl-status-run")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|v, _, _, cx| {
                                        v.run_jit();
                                        cx.notify();
                                    }))
                                    .child(format!("▶ Run (#{})", self.run_count)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(format!("Ln {}, Col 1", self.selected_line + 1))
                            .child("UTF-8")
                            .child("K (k lsp)"),
                    ),
            )
    }
}
