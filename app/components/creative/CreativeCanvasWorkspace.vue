<script setup lang="ts">
import { useCreativeCanvasWorkspace } from './useCreativeCanvasWorkspace'

const props = defineProps<{ workspaceId: string }>()
const {
  projectId, graphId, selectedNodeId, compareNodeIds, graphText,
  selectedInputsText, selectedBindingId, projectState, lastResult, templates,
  templateId, templateDescription, instantiateGraphId, replacementsText,
  bindingOverridesText, pending, pan, parsedGraph, selectedNode,
  comparedNodes, qaFindings, loadProject, loadGraph, validateGraph,
  saveTemplate, listTemplates, instantiateTemplate, selectNode,
  applyNodeEdits, toggleCompare, nodePosition, beginPan, movePan, endPan, zoom
} = useCreativeCanvasWorkspace(() => props.workspaceId)
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-3">
    <div class="grid gap-2 lg:grid-cols-[1fr_1fr_auto_auto_auto]">
      <UInput
        v-model="projectId"
        placeholder="Creative project ID"
        icon="i-lucide-folder-kanban"
      />
      <UInput
        v-model="graphId"
        placeholder="Graph ID"
        icon="i-lucide-network"
      />
      <UButton
        label="Load project"
        icon="i-lucide-refresh-cw"
        color="neutral"
        variant="outline"
        :loading="pending"
        @click="loadProject"
      />
      <UButton
        label="Load graph"
        icon="i-lucide-download"
        color="neutral"
        variant="outline"
        :loading="pending"
        @click="loadGraph"
      />
      <UButton
        label="Validate"
        icon="i-lucide-shield-check"
        :loading="pending"
        @click="validateGraph"
      />
    </div>

    <div class="grid min-h-[620px] flex-1 gap-3 xl:grid-cols-[minmax(0,1fr)_360px]">
      <div class="relative overflow-hidden rounded-xl border border-default bg-elevated/40">
        <div class="absolute left-3 top-3 z-20 flex gap-1 rounded-lg border border-default bg-default/90 p-1 shadow-sm">
          <UButton
            icon="i-lucide-minus"
            color="neutral"
            variant="ghost"
            size="xs"
            @click="zoom(-0.1)"
          />
          <span class="min-w-12 px-1 text-center text-xs leading-8 text-muted">{{ Math.round(pan.zoom * 100) }}%</span>
          <UButton
            icon="i-lucide-plus"
            color="neutral"
            variant="ghost"
            size="xs"
            @click="zoom(0.1)"
          />
          <UButton
            icon="i-lucide-locate-fixed"
            color="neutral"
            variant="ghost"
            size="xs"
            @click="Object.assign(pan, { x: 24, y: 24, zoom: 1 })"
          />
        </div>
        <div
          class="absolute inset-0 cursor-grab bg-[radial-gradient(circle,var(--ui-border)_1px,transparent_1px)] bg-[size:24px_24px] active:cursor-grabbing"
          @pointerdown="beginPan"
          @pointermove="movePan"
          @pointerup="endPan"
          @pointerleave="endPan"
        >
          <div
            class="absolute origin-top-left transition-transform duration-75"
            :style="{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${pan.zoom})` }"
          >
            <button
              v-for="(node, index) in parsedGraph?.nodes ?? []"
              :key="node.node_id"
              data-graph-node
              class="absolute w-56 rounded-xl border bg-default p-3 text-left shadow-sm transition hover:border-primary"
              :class="selectedNodeId === node.node_id ? 'border-primary ring-2 ring-primary/20' : 'border-default'"
              :style="nodePosition(index)"
              @click.stop="selectNode(node.node_id)"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <p class="truncate text-sm font-semibold text-highlighted">
                    {{ node.node_id }}
                  </p>
                  <p class="mt-0.5 text-xs text-muted">
                    {{ node.kind }}
                  </p>
                </div>
                <UButton
                  :icon="compareNodeIds.includes(node.node_id) ? 'i-lucide-git-compare-arrows' : 'i-lucide-square-dashed'"
                  color="neutral"
                  variant="ghost"
                  size="xs"
                  @click.stop="toggleCompare(node.node_id)"
                />
              </div>
              <div class="mt-3 flex items-center justify-between text-[10px] uppercase tracking-wide text-dimmed">
                <span>in {{ parsedGraph?.edges.filter(edge => edge.to_node === node.node_id).length ?? 0 }}</span>
                <span>out {{ parsedGraph?.edges.filter(edge => edge.from_node === node.node_id).length ?? 0 }}</span>
              </div>
              <div
                v-if="node.execution_binding_id"
                class="mt-2 truncate rounded bg-elevated px-2 py-1 font-mono text-[10px] text-muted"
              >
                {{ node.execution_binding_id }}
              </div>
            </button>
          </div>
        </div>
      </div>

      <div class="min-h-0 space-y-3 overflow-y-auto">
        <UCard>
          <template #header>
            <div class="font-medium">
              Execution
            </div>
          </template>
          <div class="space-y-2 text-sm text-muted">
            <p>
              Heavy graph execution and reruns run through the foreground operator CLI, not the MCP request path.
            </p>
            <p class="text-xs">
              Validate, edit, save, and instantiate graphs here. Run the resulting graph with <code>ai-tools creative</code> from an operator shell.
            </p>
          </div>
        </UCard>

        <UCard>
          <template #header>
            <div class="font-medium">
              Selected node
            </div>
          </template>
          <div
            v-if="selectedNode"
            class="space-y-2"
          >
            <p class="text-xs text-muted">
              {{ selectedNode.node_id }} · {{ selectedNode.kind }}
            </p>
            <UInput
              v-model="selectedBindingId"
              placeholder="Execution binding ID (optional)"
            />
            <UTextarea
              v-model="selectedInputsText"
              :rows="10"
              class="font-mono text-xs"
            />
            <UButton
              label="Apply local edit"
              icon="i-lucide-pencil"
              size="sm"
              block
              @click="applyNodeEdits"
            />
          </div>
          <p
            v-else
            class="text-sm text-muted"
          >
            Select a graph node to inspect or edit its typed input payload.
          </p>
        </UCard>

        <UCard v-if="comparedNodes.length">
          <template #header>
            <div class="font-medium">
              Compare
            </div>
          </template>
          <div
            class="grid gap-2"
            :class="comparedNodes.length > 1 ? 'grid-cols-2' : ''"
          >
            <pre
              v-for="node in comparedNodes"
              :key="node.node_id"
              class="max-h-56 overflow-auto rounded bg-elevated p-2 text-[10px]"
            >{{ JSON.stringify(node, null, 2) }}</pre>
          </div>
        </UCard>

        <UCard>
          <template #header>
            <div class="font-medium">
              Templates
            </div>
          </template>
          <div class="space-y-2">
            <UInput
              v-model="templateId"
              placeholder="Template ID"
            />
            <UInput
              v-model="templateDescription"
              placeholder="Description"
            />
            <div class="flex gap-2">
              <UButton
                label="Save"
                size="sm"
                @click="saveTemplate"
              /><UButton
                label="Refresh"
                size="sm"
                color="neutral"
                variant="outline"
                @click="listTemplates"
              />
            </div>
            <UInput
              v-model="instantiateGraphId"
              placeholder="New graph ID"
            />
            <UTextarea
              v-model="replacementsText"
              :rows="3"
              placeholder="Identity replacements JSON"
              class="font-mono text-xs"
            />
            <UTextarea
              v-model="bindingOverridesText"
              :rows="3"
              placeholder="Node binding overrides JSON"
              class="font-mono text-xs"
            />
            <div
              v-for="template in templates"
              :key="String(template.template_id)"
              class="flex items-center justify-between gap-2 rounded border border-default p-2"
            >
              <div class="min-w-0">
                <p class="truncate text-xs font-medium">
                  {{ template.template_id }}
                </p><p class="truncate text-[10px] text-muted">
                  v{{ template.version }}
                </p>
              </div>
              <UButton
                label="Instantiate"
                size="xs"
                color="neutral"
                variant="outline"
                @click="instantiateTemplate(String(template.template_id))"
              />
            </div>
          </div>
        </UCard>
      </div>
    </div>

    <div class="grid gap-3 xl:grid-cols-2">
      <UCard>
        <template #header>
          <div class="flex items-center justify-between">
            <span class="font-medium">Graph JSON</span><span
              v-if="!parsedGraph"
              class="text-xs text-error"
            >Invalid JSON</span>
          </div>
        </template>
        <UTextarea
          v-model="graphText"
          :rows="14"
          class="font-mono text-xs"
        />
      </UCard>
      <UCard>
        <template #header>
          <div class="font-medium">
            Project QA / last result
          </div>
        </template>
        <div class="space-y-3">
          <div
            v-if="qaFindings.length"
            class="max-h-48 space-y-1 overflow-y-auto"
          >
            <div
              v-for="finding in qaFindings"
              :key="String(finding.finding_id)"
              class="rounded border border-default p-2 text-xs"
            >
              <div class="flex justify-between gap-2">
                <span class="font-medium">{{ finding.domain }}</span><span class="text-muted">{{ finding.severity }}</span>
              </div>
              <p class="mt-1 text-muted">
                {{ finding.message }}
              </p>
            </div>
          </div>
          <pre class="max-h-72 overflow-auto rounded bg-elevated p-3 text-[10px]">{{ JSON.stringify(lastResult ?? projectState ?? {}, null, 2) }}</pre>
        </div>
      </UCard>
    </div>
  </div>
</template>
