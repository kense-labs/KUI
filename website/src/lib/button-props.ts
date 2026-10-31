import type { Locale } from "../i18n/ui";
import { useTranslations } from "../i18n/utils";

export function buttonPropRows(locale: Locale) {
  const t = useTranslations(locale);
  return [
    {
      name: "new(id)",
      type: "impl Into<ElementId>",
      default: undefined,
      description: t("button.props.new"),
    },
    {
      name: "label",
      type: "impl Into<SharedString>",
      default: '""',
      description: t("button.props.label"),
    },
    {
      name: "primary / secondary / ghost / danger",
      type: "ButtonVariant",
      default: "primary",
      description: t("button.props.variant"),
    },
    {
      name: "with_size",
      type: "Size · Sm | Md | Lg",
      default: "Md",
      description: t("button.props.size"),
    },
    {
      name: "loading",
      type: "bool",
      default: "false",
      description: t("button.props.loading"),
    },
    {
      name: "disabled",
      type: "bool",
      default: "false",
      description: t("button.props.disabled"),
    },
    {
      name: "on_click",
      type: "Fn(&ClickEvent, &mut Window, &mut App)",
      default: undefined,
      description: t("button.props.on_click"),
    },
    {
      name: "into_themed(cx)",
      type: "&App → Stateful<Div>",
      default: undefined,
      description: t("button.props.into_themed"),
    },
  ];
}

export function propsHeaders(locale: Locale) {
  const t = useTranslations(locale);
  return {
    prop: t("button.props.prop"),
    type: t("button.props.type"),
    default: t("button.props.default"),
    description: t("button.props.desc"),
  };
}
