/** Shared docs snippets — Button API is host-agnostic. */

export const installShell = `cargo install --path tools/cli
kui init
kui add button`;

export const buttonUsageRust = `use kui::components::button::Button;

Button::new("save")
    .primary()
    .label("Save")
    .into_themed(cx);`;
