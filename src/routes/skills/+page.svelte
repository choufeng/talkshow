<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import Dialog from '$lib/components/ui/dialog/index.svelte';
  import Toggle from '$lib/components/ui/toggle/index.svelte';
  import DialogFooter from '$lib/components/ui/dialog-footer/index.svelte';
  import { Plus, Pencil, Trash2 } from 'lucide-svelte';
  import type { Skill, AppConfig } from '$lib/stores/config';
  import { updateFeature, invokeWithError } from '$lib/ai/shared';
  import { createDialogState } from '$lib/hooks';

  const editDialog = createDialogState({
    onReset: () => {
      editingSkill = null;
      editForm = { name: '', description: '', prompt: '' };
    }
  });
  const deleteDialog = createDialogState({
    onReset: () => { pendingDeleteId = ''; }
  });
  let editingSkill = $state<Skill | null>(null);
  let editForm = $state({ name: '', description: '', prompt: '' });
  let pendingDeleteId = $state('');
  let isAddMode = $state(false);

  onMount(() => {
    config.load();
  });

  function handleSkillToggle(skillId: string, enabled: boolean) {
    config.save(updateFeature($config, 'skills', (f) => ({
      ...(f as NonNullable<typeof f>),
      skills: (f as NonNullable<typeof f>).skills.map((s: Skill) =>
        s.id === skillId ? { ...s, enabled } : s
      ),
      enabled: true
    })));
  }

  function openAddDialog() {
    isAddMode = true;
    editForm = { name: '', description: '', prompt: '' };
    editDialog.open();
  }

  function openEditDialog(skill: Skill) {
    isAddMode = false;
    editingSkill = skill;
    editForm = { name: skill.name, description: skill.description, prompt: skill.prompt };
    editDialog.open();
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
        editable: true,
        enabled: true
      };
      await invokeWithError('add_skill', { skill: newSkill });
    } else if (editingSkill) {
      const updated: Skill = {
        ...editingSkill,
        name: editForm.name.trim(),
        description: editForm.description.trim(),
        prompt: editForm.prompt.trim()
      };
      await invokeWithError('update_skill', { skill: updated });
    }
    await config.load();
    editDialog.close();
  }

  function openDeleteConfirm(skillId: string) {
    pendingDeleteId = skillId;
    deleteDialog.open();
  }

  async function confirmDelete() {
    await invokeWithError('delete_skill', { skillId: pendingDeleteId });
    await config.load();
    deleteDialog.close();
  }

  let canSave = $derived(
    editForm.name.trim() !== '' &&
    editForm.description.trim() !== '' &&
    editForm.prompt.trim() !== ''
  );
</script>

<div class="max-w-[800px]">
  <h2 class="text-title font-semibold text-foreground m-0 mb-8">技能设置</h2>

  <section>
    <div class="flex items-center justify-between mb-2.5">
      <div class="text-caption text-muted-foreground uppercase tracking-wider">技能列表</div>
      <button
        class="inline-flex items-center gap-1 text-body text-accent-foreground hover:underline"
        onclick={openAddDialog}
      >
        <Plus class="h-4 w-4" />
        添加自定义 Skill
      </button>
    </div>

    <div class="space-y-3">
      {#each $config.features.skills.skills || [] as skill (skill.id)}
        <div class="rounded-xl border border-border bg-background-alt p-5">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <Toggle
                checked={skill.enabled}
                onCheckedChange={(enabled) => handleSkillToggle(skill.id, enabled)}
                size="sm"
                ariaLabel={`启用 ${skill.name}`}
              />
              <div>
                <div class="flex items-center gap-1.5">
                  <span class="text-[15px] font-semibold text-foreground">{skill.name}</span>
                  <span class="inline-flex items-center rounded px-2 py-0.5 text-[11px] font-medium {skill.builtin ? 'bg-accent text-accent-foreground' : 'bg-muted text-foreground-alt'}">
                    {skill.builtin ? '预置' : '自定义'}
                  </span>
                </div>
                <div class="text-body text-foreground-alt mt-0.5">{skill.description}</div>
              </div>
            </div>
            <div class="flex items-center gap-1">
              <button
                class="rounded p-1.5 text-muted-foreground hover:text-foreground bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to shadow-btn-secondary transition-colors"
                onclick={() => openEditDialog(skill)}
                title="编辑"
              >
                <Pencil class="h-3.5 w-3.5" />
              </button>
              {#if !skill.builtin}
                <button
                  class="rounded p-1.5 text-muted-foreground hover:text-destructive bg-gradient-to-b from-btn-destructive-from/10 to-btn-destructive-to/10 shadow-btn-secondary transition-colors"
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
        <div class="rounded-lg border border-dashed border-border bg-background-alt/50 p-8 text-center">
          <p class="text-body text-muted-foreground">暂无技能，点击上方按钮添加</p>
        </div>
      {/if}
    </div>
  </section>

  <Dialog
    open={editDialog.isOpen}
    onOpenChange={editDialog.onOpenChange}
    title={isAddMode ? '添加 Skill' : '编辑 Skill'}
  >
    {#snippet children()}
      <div>
        <label for="skill-name" class="block text-body text-foreground-alt mb-1.5">名称</label>
        <input
          id="skill-name"
          class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-1 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：语气词剔除"
          bind:value={editForm.name}
        />
      </div>
      <div>
        <label for="skill-desc" class="block text-body text-foreground-alt mb-1.5">描述</label>
        <input
          id="skill-desc"
          class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-1 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="简短描述功能"
          bind:value={editForm.description}
        />
      </div>
      <div>
        <label for="skill-prompt" class="block text-body text-foreground-alt mb-1.5">Prompt 内容</label>
        <textarea
          id="skill-prompt"
          class="flex min-h-[140px] w-full rounded-md border border-border-input bg-background px-3 py-3 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1 resize-y"
          placeholder="输入发送给 LLM 的指令性 prompt"
          bind:value={editForm.prompt}
        ></textarea>
      </div>
    {/snippet}

    {#snippet footer()}
      <DialogFooter
        onCancel={() => editDialog.close()}
        onConfirm={handleSave}
        confirmText="保存"
        confirmDisabled={!canSave}
      />
    {/snippet}
  </Dialog>

  <Dialog
    open={deleteDialog.isOpen}
    onOpenChange={deleteDialog.onOpenChange}
    title="删除 Skill"
    description="确定要删除该 Skill 吗？此操作无法撤销。"
  >
    {#snippet footer()}
      <DialogFooter
        onCancel={() => deleteDialog.close()}
        onConfirm={confirmDelete}
        confirmText="删除"
        confirmVariant="danger"
      />
    {/snippet}
  </Dialog>
</div>
