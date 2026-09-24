<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import Toggle from '$lib/components/ui/toggle/index.svelte';
  import { updateFeature, invokeWithError } from '$lib/ai/shared';
  import type { HooksConfig, Skill } from '$lib/stores/config';
  import { DIDA365_PRESET } from '$lib/stores/config';
  import { Zap } from 'lucide-svelte';

  let form = $state<HooksConfig>({
    enabled: false,
    url: '',
    headers: '',
    body_template: '{"raw":"{{raw}}","text":"{{text}}","ts":"{{ts}}","app":"{{app}}"}',
    prompt: '',
    skill_ids: []
  });
  let saving = $state(false);
  let saved = $state(false);

  onMount(async () => {
    await config.load();
    if ($config.features?.hooks) {
      form = { ...$config.features.hooks };
    }
  });

  function applyDida365Preset() {
    form.url = DIDA365_PRESET.url;
    form.headers = DIDA365_PRESET.headers;
    form.body_template = DIDA365_PRESET.body_template;
    form.prompt = DIDA365_PRESET.prompt;
  }

  function toggleSkill(skillId: string) {
    form.skill_ids = form.skill_ids.includes(skillId)
      ? form.skill_ids.filter((id) => id !== skillId)
      : [...form.skill_ids, skillId];
  }

  async function handleSave() {
    saving = true;
    saved = false;
    try {
      // 复制一份纯净配置,避免把 store 响应式代理存进去
      const hooks: HooksConfig = {
        enabled: form.enabled,
        url: form.url.trim(),
        headers: form.headers,
        body_template: form.body_template,
        prompt: form.prompt,
        skill_ids: [...form.skill_ids]
      };
      await config.save(updateFeature($config, 'hooks', () => hooks));
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } finally {
      saving = false;
    }
  }

  let canSave = $derived(!form.enabled || form.url.trim() !== '');
  const skills = $derived(($config.features?.skills?.skills ?? []) as Skill[]);
</script>

<div class="max-w-[640px]">
  <h2 class="text-title font-semibold text-foreground m-0 mb-8">Hook 扩展</h2>

  <section class="mb-8">
    <div class="rounded-xl border border-border bg-background-alt p-5">
      <div class="flex items-center justify-between gap-4">
        <div>
          <div class="text-[15px] font-semibold text-foreground mb-1">语音记录</div>
          <div class="text-body text-foreground-alt">
            开启语义路由后，说"记录…"开头的内容，会自动整理并发送到下方配置的后端（如滴答清单）。需要已配置润色模型。
          </div>
        </div>
        <Toggle checked={form.enabled} onCheckedChange={(e) => (form.enabled = e)} ariaLabel="启用语音记录" />
      </div>
    </div>
  </section>

  {#if form.enabled}
    <section class="mb-8">
      <div class="text-caption text-muted-foreground uppercase tracking-wider mb-3">后端配置</div>
      <div class="rounded-xl border border-border bg-background-alt p-5 space-y-5">
        <div class="flex items-center gap-3">
          <button
            class="inline-flex items-center gap-1.5 rounded-lg border border-border px-3 py-1.5 text-body text-foreground hover:bg-muted/50 transition-colors"
            onclick={applyDida365Preset}
          >
            <Zap size={14} />
            填入滴答清单预设
          </button>
          <span class="text-caption text-muted-foreground">令牌在 developer.dida365.com 创建应用后生成</span>
        </div>

        <div>
          <label for="hook-url" class="block text-body text-foreground-alt mb-1.5">请求 URL</label>
          <input
            id="hook-url"
            class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-1 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
            type="text"
            placeholder="https://api.dida365.com/open/v1/task"
            bind:value={form.url}
          />
        </div>

        <div>
          <label for="hook-headers" class="block text-body text-foreground-alt mb-1.5">请求头（每行一个 Key: Value）</label>
          <textarea
            id="hook-headers"
            class="flex min-h-[64px] w-full rounded-md border border-border-input bg-background px-3 py-2 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1 resize-y font-mono"
            placeholder={'Authorization: Bearer 你的令牌'}
            bind:value={form.headers}
          ></textarea>
        </div>

        <div>
          <label for="hook-template" class="block text-body text-foreground-alt mb-1.5">Body 模板（JSON，支持 <code>{'{raw}'}</code> <code>{'{text}'}</code> <code>{'{ts}'}</code> <code>{'{app}'}</code>）</label>
          <textarea
            id="hook-template"
            class="flex min-h-[72px] w-full rounded-md border border-border-input bg-background px-3 py-2 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1 resize-y font-mono"
            bind:value={form.body_template}
          ></textarea>
        </div>
      </div>
    </section>

    <section class="mb-8">
      <div class="text-caption text-muted-foreground uppercase tracking-wider mb-3">内容整理</div>
      <div class="rounded-xl border border-border bg-background-alt p-5 space-y-5">
        <div>
          <label for="hook-prompt" class="block text-body text-foreground-alt mb-1.5">整理指令（可选，发送前由 LLM 整理）</label>
          <textarea
            id="hook-prompt"
            class="flex min-h-[72px] w-full rounded-md border border-border-input bg-background px-3 py-2 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1 resize-y"
            placeholder="如：提炼成一句任务标题"
            bind:value={form.prompt}
          ></textarea>
        </div>

        {#if skills.length > 0}
          <div>
            <div class="text-body text-foreground-alt mb-2">关联技能（其规则将参与整理）</div>
            <div class="flex flex-wrap gap-2">
              {#each skills as skill (skill.id)}
                <button
                  class="rounded-full border px-3 py-1 text-caption transition-colors {form.skill_ids.includes(skill.id)
                    ? 'border-btn-primary-to bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white'
                    : 'border-border bg-background text-foreground hover:bg-muted/50'}"
                  onclick={() => toggleSkill(skill.id)}
                >
                  {skill.name}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </section>
  {/if}

  <div class="flex items-center gap-3">
    <button
      class="rounded-lg bg-gradient-to-b from-btn-primary-from to-btn-primary-to px-5 py-2 text-body font-medium text-white shadow-btn-primary transition-opacity disabled:opacity-50"
      disabled={!canSave || saving}
      onclick={handleSave}
    >
      {saving ? '保存中…' : '保存'}
    </button>
    {#if saved}
      <span class="text-body text-accent-foreground">已保存</span>
    {/if}
    {#if !canSave}
      <span class="text-caption text-muted-foreground">启用后需填写 URL</span>
    {/if}
  </div>
</div>
