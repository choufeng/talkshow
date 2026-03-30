<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import GroupedSelect from '$lib/components/ui/select/index.svelte';
  import Dialog from '$lib/components/ui/dialog/index.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Plus, Pencil, Trash2 } from 'lucide-svelte';
  import type { Skill, SkillsConfig, AppConfig, ProviderConfig, ModelConfig } from '$lib/stores/config';

  let showEditDialog = $state(false);
  let showDeleteConfirm = $state(false);
  let editingSkill = $state<Skill | null>(null);
  let editForm = $state({ name: '', description: '', prompt: '' });
  let pendingDeleteId = $state('');
  let isAddMode = $state(false);

  onMount(() => {
    config.load();
  });

  function handleGlobalToggle(enabled: boolean) {
    const newConfig: AppConfig = {
      ...$config,
      features: {
        ...$config.features,
        skills: { ...$config.features.skills, enabled }
      }
    };
    config.save(newConfig);
  }

  function handleSkillToggle(skillId: string, enabled: boolean) {
    const newSkills = $config.features.skills.skills.map((s: Skill) =>
      s.id === skillId ? { ...s, enabled } : s
    );
    const newConfig: AppConfig = {
      ...$config,
      features: {
        ...$config.features,
        skills: { ...$config.features.skills, skills: newSkills }
      }
    };
    config.save(newConfig);
  }

  function buildProviderGroups() {
    return ($config.ai.providers || [])
      .filter((p: ProviderConfig) => p.type !== 'sensevoice')
      .map((p: ProviderConfig) => ({
        label: p.name,
        items: [{ value: p.id, label: p.name }]
      }));
  }

  function buildModelGroups() {
    const providerId = $config.features.skills.provider_id;
    const provider = ($config.ai.providers || []).find((p: ProviderConfig) => p.id === providerId);
    if (!provider) return [];
    return [{
      label: provider.name,
      items: (provider.models || []).map((m: ModelConfig) => ({
        value: m.name,
        label: m.name
      }))
    }];
  }

  function handleProviderChange(providerId: string) {
    const provider = ($config.ai.providers || []).find((p: ProviderConfig) => p.id === providerId);
    const firstModel = provider?.models?.[0]?.name || '';
    const newConfig: AppConfig = {
      ...$config,
      features: {
        ...$config.features,
        skills: { ...$config.features.skills, provider_id: providerId, model: firstModel }
      }
    };
    config.save(newConfig);
  }

  function handleModelChange(model: string) {
    const newConfig: AppConfig = {
      ...$config,
      features: {
        ...$config.features,
        skills: { ...$config.features.skills, model }
      }
    };
    config.save(newConfig);
  }

  function openAddDialog() {
    isAddMode = true;
    editForm = { name: '', description: '', prompt: '' };
    showEditDialog = true;
  }

  function openEditDialog(skill: Skill) {
    isAddMode = false;
    editingSkill = skill;
    editForm = { name: skill.name, description: skill.description, prompt: skill.prompt };
    showEditDialog = true;
  }

  function handleEditDialogClose(open: boolean) {
    if (!open) {
      showEditDialog = false;
      editingSkill = null;
      editForm = { name: '', description: '', prompt: '' };
    }
  }

  async function handleSave() {
    if (!editForm.name.trim() || !editForm.description.trim() || !editForm.prompt.trim()) return;

    if (isAddMode) {
      const newSkill: Skill = {
        id: crypto.randomUUID(),
        name: editForm.name.trim(),
        description: editForm.description.trim(),
        prompt: editForm.prompt.trim(),
        builtin: false,
        enabled: true
      };
      await invoke('add_skill', { skill: newSkill });
    } else if (editingSkill) {
      const updated: Skill = {
        ...editingSkill,
        name: editForm.name.trim(),
        description: editForm.description.trim(),
        prompt: editForm.prompt.trim()
      };
      await invoke('update_skill', { skill: updated });
    }
    await config.load();
    showEditDialog = false;
    editingSkill = null;
    editForm = { name: '', description: '', prompt: '' };
  }

  function openDeleteConfirm(skillId: string) {
    pendingDeleteId = skillId;
    showDeleteConfirm = true;
  }

  async function confirmDelete() {
    await invoke('delete_skill', { skillId: pendingDeleteId });
    await config.load();
    showDeleteConfirm = false;
    pendingDeleteId = '';
  }

  function handleDeleteDialogClose(open: boolean) {
    if (!open) {
      showDeleteConfirm = false;
      pendingDeleteId = '';
    }
  }

  let canSave = $derived(
    editForm.name.trim() !== '' &&
    editForm.description.trim() !== '' &&
    editForm.prompt.trim() !== ''
  );
</script>

<div class="max-w-[800px]">
  <h2 class="text-xl font-semibold text-foreground m-0 mb-6">技能设置</h2>

  <section class="mb-7">
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">全局</div>
    <div class="rounded-lg border border-border bg-background-alt p-3.5">
      <div class="flex items-center justify-between">
        <div>
          <div class="text-[13px] font-semibold text-foreground">Skills 功能</div>
          <div class="text-[11px] text-foreground-alt">启用后，转写文字将自动经过 Skill 处理管线</div>
        </div>
        <button
          class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-2 {$config.features.skills.enabled ? 'bg-accent-foreground' : 'bg-border'}"
          role="switch"
          aria-checked={$config.features.skills.enabled}
          onclick={() => handleGlobalToggle(!$config.features.skills.enabled)}
        >
          <span class="pointer-events-none inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {$config.features.skills.enabled ? 'translate-x-4' : 'translate-x-0'}"></span>
        </button>
      </div>
    </div>
  </section>

  <section class="mb-7">
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">LLM 服务</div>
    <div class="rounded-lg border border-border bg-background-alt p-3.5">
      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="block text-[11px] text-foreground-alt mb-1">Provider</label>
          <GroupedSelect
            value={$config.features.skills.provider_id}
            groups={buildProviderGroups()}
            placeholder="选择 Provider"
            onChange={handleProviderChange}
          />
        </div>
        <div>
          <label class="block text-[11px] text-foreground-alt mb-1">Model</label>
          <GroupedSelect
            value={$config.features.skills.model}
            groups={buildModelGroups()}
            placeholder="选择模型"
            onChange={handleModelChange}
          />
        </div>
      </div>
    </div>
  </section>

  <section>
    <div class="flex items-center justify-between mb-2.5">
      <div class="text-[11px] text-muted-foreground uppercase tracking-wider">技能列表</div>
      <button
        class="inline-flex items-center gap-1 text-xs text-accent-foreground hover:underline"
        onclick={openAddDialog}
      >
        <Plus class="h-3.5 w-3.5" />
        添加自定义 Skill
      </button>
    </div>

    <div class="space-y-2">
      {#each $config.features.skills.skills || [] as skill (skill.id)}
        <div class="rounded-lg border border-border bg-background-alt p-3.5">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <button
                class="relative inline-flex h-4.5 w-8 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-2 {skill.enabled ? 'bg-accent-foreground' : 'bg-border'}"
                role="switch"
                aria-checked={skill.enabled}
                onclick={() => handleSkillToggle(skill.id, !skill.enabled)}
              >
                <span class="pointer-events-none inline-block h-3 w-3 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {skill.enabled ? 'translate-x-3.5' : 'translate-x-0'}"></span>
              </button>
              <div>
                <div class="flex items-center gap-1.5">
                  <span class="text-[13px] font-semibold text-foreground">{skill.name}</span>
                  <span class="inline-flex items-center rounded px-1.5 py-0.5 text-[9px] font-medium {skill.builtin ? 'bg-accent text-accent-foreground' : 'bg-muted text-foreground-alt'}">
                    {skill.builtin ? '预置' : '自定义'}
                  </span>
                </div>
                <div class="text-[11px] text-foreground-alt mt-0.5">{skill.description}</div>
              </div>
            </div>
            <div class="flex items-center gap-1">
              <button
                class="rounded p-1.5 text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
                onclick={() => openEditDialog(skill)}
                title="编辑"
              >
                <Pencil class="h-3.5 w-3.5" />
              </button>
              {#if !skill.builtin}
                <button
                  class="rounded p-1.5 text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
                  onclick={() => openDeleteConfirm(skill.id)}
                  title="删除"
                >
                  <Trash2 class="h-3.5 w-3.5" />
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/each}

      {#if ($config.features.skills.skills || []).length === 0}
        <div class="rounded-lg border border-dashed border-border bg-background-alt/50 p-6 text-center">
          <p class="text-xs text-muted-foreground">暂无技能，点击上方按钮添加</p>
        </div>
      {/if}
    </div>
  </section>

  <Dialog
    open={showEditDialog}
    onOpenChange={handleEditDialogClose}
    title={isAddMode ? '添加 Skill' : '编辑 Skill'}
  >
    {#snippet children()}
      <div>
        <label for="skill-name" class="block text-[11px] text-foreground-alt mb-1">名称</label>
        <input
          id="skill-name"
          class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：语气词剔除"
          bind:value={editForm.name}
        />
      </div>
      <div>
        <label for="skill-desc" class="block text-[11px] text-foreground-alt mb-1">描述</label>
        <input
          id="skill-desc"
          class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="简短描述功能"
          bind:value={editForm.description}
        />
      </div>
      <div>
        <label for="skill-prompt" class="block text-[11px] text-foreground-alt mb-1">Prompt 内容</label>
        <textarea
          id="skill-prompt"
          class="flex min-h-[120px] w-full rounded-md border border-border-input bg-background px-3 py-2 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1 resize-y"
          placeholder="输入发送给 LLM 的指令性 prompt"
          bind:value={editForm.prompt}
        ></textarea>
      </div>
    {/snippet}

    {#snippet footer()}
      <button
        type="button"
        class="rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-muted transition-colors"
        onclick={() => handleEditDialogClose(false)}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-foreground px-3 py-1.5 text-xs text-background hover:bg-foreground/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        onclick={handleSave}
        disabled={!canSave}
      >
        保存
      </button>
    {/snippet}
  </Dialog>

  <Dialog
    open={showDeleteConfirm}
    onOpenChange={handleDeleteDialogClose}
    title="删除 Skill"
    description="确定要删除该 Skill 吗？此操作无法撤销。"
  >
    {#snippet footer()}
      <button
        type="button"
        class="rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-muted transition-colors"
        onclick={() => handleDeleteDialogClose(false)}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-destructive px-3 py-1.5 text-xs text-white hover:bg-destructive/90 transition-colors"
        onclick={confirmDelete}
      >
        删除
      </button>
    {/snippet}
  </Dialog>
</div>
