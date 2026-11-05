"""Update component audit evidence and render the user's desktop progress report."""
import argparse
from datetime import datetime
import json
from pathlib import Path
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parents[1]
STATE = ROOT / "docs/audits/component-audit-status.json"
STAGES = ("source_reviewed", "native_wasm_passed", "install_passed", "visual_accepted", "motion_accepted")


def complete(row):
    return row["user_default_pass"] or all(row.get(stage, False) for stage in STAGES)


def render(data):
    rows = data["components"]
    done = [r for r in rows if complete(r)]
    partial = [r for r in rows if not complete(r) and any(r.get(s) for s in STAGES)]
    pending = [r for r in rows if r not in done and r not in partial]
    lines = [
        "# KUI 组件验收进度", "", f"最后更新：{data['updated']}（北京时间）", "",
        f"**共 {len(rows)} 项：已通过 {len(done)} 项，检查中 {len(partial)} 项，尚未开始本轮检查 {len(pending)} 项。**", "",
        f"之前的108项按用户要求默认通过，不再复查。基线 Git 提交：`{data['baseline_commit']}`。", "",
        "其余组件逐项记录源码参数、原生/WASM、独立安装、视觉和动效。五项证据齐全才记为本轮验收通过；源码核对或编译通过不会单独计为完成。", "",
        "查看组件可点击名称；点击 ID 可查看本地实现规格。每完成一个检查阶段就更新此文件，并在末尾记录日期。", "",
    ]
    def link(row):
        return f"[{row['title']}](http://127.0.0.1:4321/kui/components/{row['id']}/)"
    def identifier(row):
        return f"[{row['id']}](<{ROOT}/docs/items/{row['id']}.md>)"
    lines += ["## 已通过", "", "| 组件 | ID | 结论 |", "| --- | --- | --- |"]
    for row in done:
        reason = "用户指定默认通过" if row["user_default_pass"] else "本轮逐项验收通过"
        lines.append(f"| {link(row)} | {identifier(row)} | {reason} |")
    lines += ["", "## 检查中", "", "✓ 已完成该阶段；— 尚未完成。", "", "| 组件 | ID | 源码参数 | 原生/WASM | 独立安装 | 视觉 | 动效 | 备注 |", "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for row in partial:
        checks = " | ".join("✓" if row.get(stage) else "—" for stage in STAGES)
        lines.append(f"| {link(row)} | {identifier(row)} | {checks} | {row['notes'].replace('|', '/')} |")
    lines += ["", "## 尚未开始本轮检查", "", "这些组件已有实现，此处表示尚未完成这一轮逐项源码与体验核对。", "", "| 组件 | ID | 状态 |", "| --- | --- | --- |"]
    for row in pending:
        lines.append(f"| {link(row)} | {identifier(row)} | 待检查 |")
    lines += ["", "## 更新记录", ""]
    for entry in reversed(data["history"]):
        lines.append(f"- {entry['date']} · {entry['component']}：{entry['note']}")
    lines += ["", "本表不等同于全站恒定60fps或逐帧误差认证。默认通过的108项是用户指定结论。", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--id", help="Component installation ID")
    parser.add_argument("--stage", choices=STAGES)
    parser.add_argument("--note")
    parser.add_argument("--output", type=Path, default=Path.home() / "Desktop/KUI 组件验收进度.md")
    args = parser.parse_args()
    if bool(args.id) != bool(args.stage):
        parser.error("--id and --stage must be provided together")
    data = json.loads(STATE.read_text())
    now = datetime.now(ZoneInfo("Asia/Shanghai")).strftime("%Y-%m-%d %H:%M")
    if args.id:
        row = next((r for r in data["components"] if r["id"] == args.id), None)
        if row is None:
            parser.error(f"unknown component: {args.id}")
        row[args.stage] = True
        row["updated"] = now
        if args.note:
            row["notes"] = args.note
        data["history"].append(dict(date=now, component=args.id, note=args.note or args.stage))
    data["updated"] = now
    STATE.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
    args.output.write_text(render(data))
    print(f"Updated {args.output}")
    print(f"Accepted {sum(complete(r) for r in data['components'])}/{len(data['components'])}")


if __name__ == "__main__":
    main()
