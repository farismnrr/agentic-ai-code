import { clientErrorMessage } from '../../utils/client-errors'

type GraphNode = {
  node_id: string
  kind: string
  execution_binding_id?: string | null
  inputs?: Record<string, unknown>
}

type GraphEdge = {
  from_node: string
  from_port: string
  to_node: string
  to_port: string
}

type CreativeGraph = {
  schema_version: number
  graph_id: string
  project_id: string
  revision: number
  nodes: GraphNode[]
  edges: GraphEdge[]
}

type CanvasApiResponse = {
  tool: 'creative_graph' | 'creative_project'
  content: unknown[]
  isError: boolean
}

export function useCreativeCanvasWorkspace(workspaceId: () => string) {
  const toast = useToast()
  const projectId = ref('')
  const graphId = ref('')
  const selectedNodeId = ref<string | null>(null)
  const compareNodeIds = ref<string[]>([])
  const graphText = ref(JSON.stringify({
    schema_version: 1,
    graph_id: 'graph_canvas',
    project_id: 'project_id_here',
    revision: 1,
    nodes: [],
    edges: []
  }, null, 2))
  const selectedInputsText = ref('{}')
  const selectedBindingId = ref('')
  const projectState = ref<Record<string, unknown> | null>(null)
  const lastResult = ref<Record<string, unknown> | null>(null)
  const templates = ref<Array<Record<string, unknown>>>([])
  const templateId = ref('')
  const templateDescription = ref('')
  const instantiateGraphId = ref('')
  const replacementsText = ref('{}')
  const bindingOverridesText = ref('{}')
  const pending = ref(false)
  const pan = reactive({ x: 24, y: 24, zoom: 1 })
  const dragging = ref(false)
  const dragStart = reactive({ x: 0, y: 0, panX: 0, panY: 0 })

  const parsedGraph = computed<CreativeGraph | null>(() => {
    try {
      const graph = JSON.parse(graphText.value) as CreativeGraph
      return graph && Array.isArray(graph.nodes) && Array.isArray(graph.edges) ? graph : null
    } catch {
      return null
    }
  })

  const selectedNode = computed(() => parsedGraph.value?.nodes.find(node => node.node_id === selectedNodeId.value) ?? null)
  const comparedNodes = computed(() => parsedGraph.value?.nodes.filter(node => compareNodeIds.value.includes(node.node_id)) ?? [])
  const qaFindings = computed(() => Array.isArray(projectState.value?.qa_findings) ? projectState.value.qa_findings as Array<Record<string, unknown>> : [])

  function parseToolPayload(response: CanvasApiResponse) {
    const text = response.content
      .map((part) => {
        if (part && typeof part === 'object' && 'text' in part && typeof (part as { text?: unknown }).text === 'string') return (part as { text: string }).text
        return ''
      })
      .find(Boolean)
    if (!text) return {}
    try {
      return JSON.parse(text) as Record<string, unknown>
    } catch {
      return { text }
    }
  }

  async function invoke(tool: CanvasApiResponse['tool'], args: Record<string, unknown>) {
    pending.value = true
    try {
      const response = await $fetch<CanvasApiResponse>('/api/creative/canvas', {
        method: 'POST',
        body: { workspaceId: workspaceId(), tool, arguments: args }
      })
      const payload = parseToolPayload(response)
      if (response.isError) throw new Error(typeof payload.message === 'string' ? payload.message : 'Creative relay returned an error')
      lastResult.value = payload
      return payload
    } catch (error) {
      toast.add({ title: 'Creative Canvas request failed', description: clientErrorMessage(error, 'The Creative relay could not complete this action.'), color: 'error' })
      throw error
    } finally {
      pending.value = false
    }
  }

  async function loadProject() {
    if (!projectId.value) return
    const payload = await invoke('creative_project', { action: 'get', project_id: projectId.value })
    projectState.value = payload.project as Record<string, unknown> ?? null
  }

  async function loadGraph() {
    if (!projectId.value || !graphId.value) return
    const payload = await invoke('creative_graph', { action: 'get', project_id: projectId.value, graph_id: graphId.value })
    if (payload.graph) {
      graphText.value = JSON.stringify(payload.graph, null, 2)
      selectNode(null)
    }
  }

  async function validateGraph() {
    const graph = parsedGraph.value
    if (!graph) return invalidGraphToast()
    const payload = await invoke('creative_graph', { action: 'validate', graph })
    const valid = payload.validation && (payload.validation as { valid?: boolean }).valid
    toast.add({ title: valid ? 'Graph valid' : 'Graph has diagnostics', color: valid ? 'success' : 'warning' })
  }

  async function saveTemplate() {
    const graph = parsedGraph.value
    if (!graph || !templateId.value) return
    const outputNodeIds = graph.nodes.filter(node => !graph.edges.some(edge => edge.from_node === node.node_id)).map(node => node.node_id)
    await invoke('creative_graph', {
      action: 'template_save',
      template_id: templateId.value,
      description: templateDescription.value,
      graph,
      input_node_ids: graph.nodes.filter(node => ['input_text', 'input_asset', 'element_ref'].includes(node.kind)).map(node => node.node_id),
      output_node_ids: outputNodeIds.length ? outputNodeIds : graph.nodes.slice(-1).map(node => node.node_id)
    })
    await listTemplates()
  }

  async function listTemplates() {
    if (!projectId.value) return
    const payload = await invoke('creative_graph', { action: 'template_list', project_id: projectId.value })
    templates.value = Array.isArray(payload.templates) ? payload.templates as Array<Record<string, unknown>> : []
  }

  async function instantiateTemplate(id: string) {
    if (!projectId.value || !instantiateGraphId.value) return
    const payload = await invoke('creative_graph', {
      action: 'template_instantiate',
      project_id: projectId.value,
      template_id: id,
      graph_id: instantiateGraphId.value,
      replacements: parseObject(replacementsText.value, 'replacements'),
      binding_overrides: parseObject(bindingOverridesText.value, 'binding overrides')
    })
    if (payload.graph) {
      graphText.value = JSON.stringify(payload.graph, null, 2)
      graphId.value = instantiateGraphId.value
    }
  }

  function parseObject(text: string, label: string) {
    try {
      const value = JSON.parse(text)
      if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error()
      return value as Record<string, unknown>
    } catch {
      throw new Error(`Invalid ${label} JSON`)
    }
  }

  function selectNode(id: string | null) {
    selectedNodeId.value = id
    const node = parsedGraph.value?.nodes.find(item => item.node_id === id)
    selectedInputsText.value = JSON.stringify(node?.inputs ?? {}, null, 2)
    selectedBindingId.value = node?.execution_binding_id ?? ''
  }

  function applyNodeEdits() {
    if (!selectedNodeId.value || !parsedGraph.value) return
    try {
      const inputs = parseObject(selectedInputsText.value, 'node inputs')
      const graph = structuredClone(parsedGraph.value)
      const node = graph.nodes.find(item => item.node_id === selectedNodeId.value)
      if (!node) return
      node.inputs = inputs
      node.execution_binding_id = selectedBindingId.value.trim() || null
      graph.revision += 1
      graphText.value = JSON.stringify(graph, null, 2)
      selectNode(node.node_id)
    } catch (error) {
      toast.add({ title: 'Invalid node edit', description: error instanceof Error ? error.message : 'Invalid node JSON', color: 'error' })
    }
  }

  function toggleCompare(nodeId: string) {
    compareNodeIds.value = compareNodeIds.value.includes(nodeId)
      ? compareNodeIds.value.filter(id => id !== nodeId)
      : [...compareNodeIds.value.slice(-1), nodeId]
  }

  function nodePosition(index: number) {
    const column = index % 4
    const row = Math.floor(index / 4)
    return { left: `${column * 260}px`, top: `${row * 180}px` }
  }

  function beginPan(event: PointerEvent) {
    if ((event.target as HTMLElement).closest('[data-graph-node]')) return
    dragging.value = true
    dragStart.x = event.clientX
    dragStart.y = event.clientY
    dragStart.panX = pan.x
    dragStart.panY = pan.y
  }

  function movePan(event: PointerEvent) {
    if (!dragging.value) return
    pan.x = dragStart.panX + event.clientX - dragStart.x
    pan.y = dragStart.panY + event.clientY - dragStart.y
  }

  function endPan() {
    dragging.value = false
  }

  function zoom(delta: number) {
    pan.zoom = Math.min(1.8, Math.max(0.5, Number((pan.zoom + delta).toFixed(2))))
  }

  function invalidGraphToast() {
    toast.add({ title: 'Graph JSON is invalid', color: 'error' })
  }

  return {
    projectId, graphId, selectedNodeId, compareNodeIds, graphText,
    selectedInputsText, selectedBindingId, projectState, lastResult, templates,
    templateId, templateDescription, instantiateGraphId, replacementsText,
    bindingOverridesText, pending, pan, dragging, parsedGraph, selectedNode,
    comparedNodes, qaFindings, loadProject, loadGraph, validateGraph,
    saveTemplate, listTemplates, instantiateTemplate, selectNode,
    applyNodeEdits, toggleCompare, nodePosition, beginPan, movePan, endPan, zoom
  }
}
