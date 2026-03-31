#!/usr/bin/env python3
"""DashScope API 延迟诊断工具"""

import time
import httpx
import json
import sys

API_KEY = (
    sys.argv[1] if len(sys.argv) > 1 else input("输入 DashScope API Key: ").strip()
)
BASE_URL = "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"

SHORT_PROMPT = "Hi"

LONG_PROMPT = """你是一个语音转文字的文本处理助手。请根据以下规则处理用户的输入文本。

当前用户正在使用的应用是：Google Chrome (com.google.Chrome)
请仅应用与当前场景相关的规则，跳过不适用的规则。
---
【语气词剔除】
去除中文口语中常见的无意义语气词和填充词，包括但不限于：
"嗯"、"啊"、"额"、"呃"、"那个"、"就是"、"然后"、"对吧"、"的话"、"怎么说呢"。
注意保留有实际语义的词语，例如"然后"在表示时间顺序时应保留。不要改变原文的语义和语气。
---
【错别字修正】
识别并修正文本中的错别字、同音错误和常见输入法导致的文字错误。
只修正明确的错误，不要对有歧义的内容做主观改动。
---
【口语润色】
保持口语化的表达风格，但使语句更流畅自然。
具体做法：去除重复表达、调整语序使其更通顺、适当添加标点使句子结构更清晰。
---
【书面格式化】
将口语化的表达转换为规范的书面表达，适合邮件、文档、报告等正式场景。
- 词汇替换：将口语化词汇替换为正式表达
- 列表结构化：将序列词转换为规范的有序列表格式
- 段落重组：识别话题转换，合理分段
- 标点规范：统一使用全角标点

请只输出处理后的纯文本。

输入文本：
下面我们来进行一个测试，看这个效果如何。"""

TESTS = [
    ("短 prompt + 默认(思考)", "qwen3.5-flash", SHORT_PROMPT, {}),
    ("短 prompt + 关闭思考", "qwen3.5-flash", SHORT_PROMPT, {"enable_thinking": False}),
    ("长 prompt + 默认(思考)", "qwen3.5-flash", LONG_PROMPT, {}),
    ("长 prompt + 关闭思考", "qwen3.5-flash", LONG_PROMPT, {"enable_thinking": False}),
]


def test_one(label: str, model: str, prompt: str, extra: dict = None):
    body = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
    }
    if extra:
        body.update(extra)

    t_dns_start = time.perf_counter()
    with httpx.Client(http2=True) as client:
        t_connect_start = time.perf_counter()
        with client.stream(
            "POST",
            BASE_URL,
            headers={
                "Authorization": f"Bearer {API_KEY}",
                "Content-Type": "application/json",
            },
            json=body,
            timeout=120,
        ) as resp:
            t_first_byte = time.perf_counter()
            resp.raise_for_status()
            full_body = resp.read()
            t_done = time.perf_counter()

    dns_ms = (t_connect_start - t_dns_start) * 1000
    ttfb_ms = (t_first_byte - t_connect_start) * 1000
    read_ms = (t_done - t_first_byte) * 1000
    total_ms = (t_done - t_dns_start) * 1000

    data = json.loads(full_body)
    content = data["choices"][0]["message"]["content"][:80]
    reasoning = data["choices"][0]["message"].get("reasoning_content", "")
    usage = data.get("usage", {})

    print(f"\n{'=' * 60}")
    print(f"  {label}  |  {model}")
    print(f"{'=' * 60}")
    print(f"  DNS + 连接建立:   {dns_ms:>8.1f} ms")
    print(f"  首字节 (TTFB):    {ttfb_ms:>8.1f} ms  ← 服务器开始响应")
    print(f"  读取响应体:       {read_ms:>8.1f} ms  ← 模型生成时间")
    print(f"  总耗时:           {total_ms:>8.1f} ms")
    print(
        f"  Token 用量:       prompt={usage.get('prompt_tokens', '?')}, completion={usage.get('completion_tokens', '?')}, thinking={len(reasoning) if reasoning else 0}chars"
    )
    if reasoning:
        print(f"  思考内容预览:     {reasoning[:100]}...")
    print(f"  响应预览:         {content}...")
    print(f"{'=' * 60}")

    return total_ms


if __name__ == "__main__":
    print("DashScope API 延迟诊断")
    print(f"目标: {BASE_URL}\n")

    results = []
    for label, model, prompt, extra in TESTS:
        try:
            ms = test_one(label, model, prompt, extra)
            results.append((label, model, ms))
        except Exception as e:
            print(f"\n[ERROR] {label} / {model}: {e}")
        time.sleep(1)

    print("\n\n汇总:")
    print(f"  {'场景':<20} {'模型':<22} {'耗时':>8}")
    print(f"  {'-' * 52}")
    for label, model, ms in results:
        print(f"  {label:<20} {model:<22} {ms:>7.0f} ms")
