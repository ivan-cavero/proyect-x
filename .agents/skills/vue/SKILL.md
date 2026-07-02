---
name: vue
description: Write modern Vue 3.5 components using the Composition API, script setup, Pinia 3 stores, composables, and Tailwind 4. Use whenever creating or editing any .vue file, writing Vue components, composables, Pinia stores, or TypeScript in the praxis dashboard. Also use when the user mentions Vue, Composition API, script setup, defineProps, defineModel, useTemplateRef, Pinia, composables, Tailwind, Vite, reactivity, ref, computed, watch, or asks about Vue 3.5 features.
---

# Vue 3.5 — Composition API + Pinia 3 + Tailwind 4

praxis's dashboard runs Vue 3.5 + Vite 8 + TypeScript 6 (strict) + Tailwind 4
+ Pinia 3. This skill covers the modern idioms for that stack.

The guiding principle: **`<script setup>` with TypeScript, composables for
reusable logic, Pinia for shared state, and the reactivity system doing the
heavy lifting.** No Options API, no `this`, no `data()` / `methods()` /
`computed` object properties.

---

## Component structure — `<script setup lang="ts">`

Every component uses `<script setup>` with TypeScript. The order is:
imports → props/emits → refs/computed → functions → lifecycle.

```vue
<script setup lang="ts">
// 1. Imports
import { ref, computed, onMounted } from 'vue'
import { useAppStore } from '../stores/app'
import Button from './ui/Button.vue'

// 2. Props + emits (typed)
const { title, count = 0 } = defineProps<{
  title: string
  count?: number
}>()
const emit = defineEmits<{
  submit: [value: string]
  cancel: []
}>()

// 3. Reactive state + computed
const store = useAppStore()
const inputValue = ref('')
const isValid = computed(() => inputValue.value.trim().length > 0)

// 4. Functions
function handleSubmit() {
  if (!isValid.value) return
  emit('submit', inputValue.value)
  inputValue.value = ''
}

// 5. Lifecycle
onMounted(() => store.refreshAll())
</script>

<template>
  <form @submit.prevent="handleSubmit">
    <h2>{{ title }}</h2>
    <input v-model="inputValue" />
    <Button type="submit" :disabled="!isValid">Submit</Button>
  </form>
</template>

<style scoped>
/* component-scoped styles */
</style>
```

---

## Vue 3.5 features — use these

### Reactive Props Destructure (stable in 3.5)

Destructured props are now reactive by default. Use native default value
syntax instead of `withDefaults`:

```vue
<script setup lang="ts">
// ✅ Vue 3.5 — reactive destructure with native defaults
const { count = 0, label = 'Default' } = defineProps<{
  count?: number
  label?: string
}>()
</script>
```

**Gotcha:** watching a destructured prop or passing it to a composable
requires a getter (the compiler transforms `count` → `props.count` on
access, but `watch(count)` captures the value, not the reference):

```ts
// ❌ Compile error — count is not a ref
watch(count, (newVal) => { ... })

// ✅ Wrap in a getter
watch(() => count, (newVal) => { ... })

// ✅ Composables should normalize with toValue()
useDynamicCount(() => count)
```

### `useTemplateRef()` (3.5)

Replaces the old `ref(null)` + matching `ref="name"` pattern. Matches by
runtime string ID, supports dynamic refs:

```vue
<script setup lang="ts">
import { useTemplateRef } from 'vue'

const inputRef = useTemplateRef('input')

function focus() {
  inputRef.value?.focus()
}
</script>

<template>
  <input ref="input" />
</template>
```

### `defineModel()` (stable in 3.4)

Two-way binding for `v-model` without manual `props` + `emit` boilerplate:

```vue
<!-- Child.vue -->
<script setup lang="ts">
const model = defineModel<string>({ required: true })
</script>

<template>
  <input v-model="model" />
</template>

<!-- Parent.vue -->
<Child v-model="username" />
```

Multiple models with named `defineModel`:

```vue
<script setup lang="ts">
const firstName = defineModel<string>('firstName')
const lastName = defineModel<string>('lastName')
</script>
```

### `useId()` (3.5)

SSR-safe unique IDs for form elements and accessibility attributes:

```vue
<script setup lang="ts">
import { useId } from 'vue'
const id = useId()
</script>

<template>
  <label :for="id">Name</label>
  <input :id="id" type="text" />
</template>
```

### `onWatcherCleanup()` (3.5)

Register cleanup callbacks in watchers — replaces the old third-argument
`onCleanup` parameter:

```ts
import { watch, onWatcherCleanup } from 'vue'

watch(selectedId, (newId) => {
  const controller = new AbortController()
  fetch(`/api/projects/${newId}`, { signal: controller.signal })
    .then((res) => res.json())
    .then((data) => { project.value = data })

  // Cleanup runs before the next watcher invocation
  onWatcherCleanup(() => controller.abort())
})
```

---

## Reactivity — the right primitive for the job

| Primitive | Use when |
|-----------|----------|
| `ref(value)` | Primitive values, single objects you replace wholesale |
| `reactive(obj)` | Objects you mutate in-place (rare in `<script setup>` — prefer `ref`) |
| `computed(() => ...)` | Derived state that caches and only recomputes when deps change |
| `shallowRef(obj)` | Large objects where deep reactivity is unnecessary (perf) |
| `shallowReactive(obj)` | Only top-level keys need to be reactive |
| `readonly(state)` | Expose state as read-only to child components |

### `ref` vs `reactive`

Prefer `ref` in `<script setup>`. It's consistent (always `.value`), works
with primitives, and destructures cleanly. Use `reactive` only for grouped
state objects:

```ts
// ✅ Default: ref for each piece of state
const isLoading = ref(false)
const projects = ref<Project[]>([])
const error = ref<string | null>(null)

// ✅ Acceptable: reactive for a cohesive state group
const form = reactive({
  name: '',
  description: '',
  isValid: false,
})
```

### `shallowRef` for performance

Large objects (full API responses, big arrays) don't need deep reactivity.
`shallowRef` only tracks `.value` reassignment, not nested mutations:

```ts
// ✅ Large list — shallowRef, replace wholesale to trigger update
const eventLog = shallowRef<Event[]>([])

function appendEvents(newEvents: Event[]) {
  eventLog.value = [...eventLog.value, ...newEvents]  // triggers reactivity
}
```

### `computed` — cache your derivations

```ts
// ✅ Computed: cached, only recomputes when projects changes
const activeProjects = computed(() =>
  projects.value.filter((p) => p.status === 'active')
)

// ❌ Function: re-runs on every template render
function getActiveProjects() {
  return projects.value.filter((p) => p.status === 'active')
}
```

---

## Composables — reusable logic

Composables encapsulate reactive logic. Name them `useXxx`, return refs and
functions:

```ts
// src/composables/useApi.ts
import { ref, type Ref } from 'vue'

export function useApi<T>(url: string) {
  const data: Ref<T | null> = ref(null)
  const error: Ref<string | null> = ref(null)
  const isLoading = ref(false)

  async function fetchAll() {
    isLoading.value = true
    error.value = null
    try {
      const response = await fetch(url)
      if (!response.ok) throw new Error(`HTTP ${response.status}`)
      data.value = await response.json() as T
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Unknown error'
    } finally {
      isLoading.value = false
    }
  }

  return { data, error, isLoading, fetchAll }
}
```

### Composable rules
- **`use` prefix**, return an object of refs + functions.
- **Accept `MaybeRefOrGetter`** for inputs that might be reactive — normalize
  with `toValue()`:

```ts
import { toValue, type MaybeRefOrGetter } from 'vue'

export function useProject(id: MaybeRefOrGetter<string>) {
  // toValue() unwraps refs, getters, and plain values
  const resolvedId = toValue(id)
  // ...
}
```

- **Clean up in `onScopeDispose`** for side effects that outlive the
  component (intervals, event listeners, WebSocket connections):

```ts
import { onScopeDispose } from 'vue'

export function useWebSocket(url: string) {
  const ws = new WebSocket(url)

  onScopeDispose(() => {
    ws.close()
  })

  return { ws }
}
```

---

## Pinia 3 — setup stores

Use the **setup store** syntax (function-based), not the options syntax:

```ts
// src/stores/app.ts
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useAppStore = defineStore('app', () => {
  // State
  const projects = ref<Project[]>([])
  const loading = ref(false)
  const activeProjectId = ref<string | null>(null)

  // Getters (computed)
  const activeProject = computed(() =>
    projects.value.find((p) => p.id === activeProjectId.value)
  )
  const projectCount = computed(() => projects.value.length)

  // Actions (functions)
  async function refreshAll() {
    loading.value = true
    try {
      const response = await fetch('/api/projects')
      projects.value = await response.json()
    } finally {
      loading.value = false
    }
  }

  function selectProject(id: string) {
    activeProjectId.value = id
  }

  return {
    // State
    projects,
    loading,
    activeProjectId,
    // Getters
    activeProject,
    projectCount,
    // Actions
    refreshAll,
    selectProject,
  }
})
```

### Store rules
- **Setup syntax** (function + `ref`/`computed`), not options syntax.
- **Return everything** the components need: state, getters, actions.
- **Don't destructure store in components** — loses reactivity. Use
  `storeToRefs` for state/getters, call actions directly:

```ts
import { storeToRefs } from 'pinia'

const store = useAppStore()

// ✅ storeToRefs preserves reactivity
const { projects, loading } = storeToRefs(store)

// ✅ Actions are fine to destructure (they're plain functions)
const { refreshAll, selectProject } = store
```

---

## TypeScript — strict, no `any`

The dashboard's `tsconfig.json` has `strict: true`, `noUnusedLocals`,
`noUnusedParameters`. Respect it.

### Typed props

```ts
// ✅ Generic syntax — type-safe, no runtime overhead
const { title, count = 0 } = defineProps<{
  title: string
  count?: number
  items?: Project[]
}>()
```

### Typed emits

```ts
const emit = defineEmits<{
  submit: [value: string]
  cancel: []
  update: [id: string, changes: Partial<Project>]
}>()

emit('submit', 'hello')           // ✅
emit('submit', 42)                // ❌ type error
```

### No `any` — use `unknown` + narrowing

```ts
// ❌ any disables type checking
catch (e: any) { console.error(e.message) }

// ✅ unknown + narrowing
catch (e: unknown) {
  const message = e instanceof Error ? e.message : String(e)
  console.error(message)
}
```

### Define component types for reuse

```ts
// types.ts
export interface Project {
  id: string
  name: string
  description: string | null
  created_at: string
  forge_toml: string | null
}

export type ViewName = 'overview' | 'projects' | 'settings'
```

---

## Tailwind 4 — utility-first + CSS variables

praxis uses Tailwind 4 via `@tailwindcss/vite`. The dashboard also uses CSS
custom properties (`--clr-*`, `--space-*`) for the design system.

### When to use what
- **Tailwind utilities** for layout, spacing, flexbox, grid — inline in the
  template.
- **CSS custom properties** for the design system colors, fonts, spacing
  scale — referenced in `<style scoped>` or inline `style="..."`.
- **`<style scoped>`** for component-specific patterns that are too complex
  for utilities.

```vue
<template>
  <!-- Tailwind for layout -->
  <div class="flex gap-2 items-center">
    <!-- CSS vars for design system colors -->
    <span class="badge" style="color: var(--clr-primary)">
      {{ status }}
    </span>
  </div>
</template>

<style scoped>
.badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  background: var(--clr-primary-glow);
}
</style>
```

### Don't fight the design system
- Use `var(--clr-*)` for colors, not hardcoded hex values.
- Use `var(--space-*)` for spacing scale.
- Use `var(--font-mono)` for monospace text.

---

## Async — Promise chains, not async/await

Per the project's AGENTS.md: **Promise chains, not `async/await`.** This is a
deliberate style choice for the dashboard.

```ts
// ❌ async/await — banned by convention
async function loadProjects() {
  const res = await fetch('/api/projects')
  const data = await res.json()
  projects.value = data
}

// ✅ Promise chain
function loadProjects() {
  return fetch('/api/projects')
    .then((res) => res.json())
    .then((data) => { projects.value = data })
    .catch((err: unknown) => {
      error.value = err instanceof Error ? err.message : 'Request failed'
    })
    .finally(() => { loading.value = false })
}
```

---

## Performance patterns

### `v-memo` for expensive list items

```vue
<div v-for="item in largeList" :key="item.id" v-memo="[item.id, item.updated]">
  <ExpensiveItem :item="item" />
</div>
```

`v-memo` skips re-rendering the item unless the memoized deps change.

### `shallowRef` for large API responses

```ts
const eventLog = shallowRef<Event[]>([])  // don't deep-track 1000 events
```

### Lazy components for route-level splitting

```ts
import { defineAsyncComponent } from 'vue'

const SettingsView = defineAsyncComponent(() => import('./views/SettingsView.vue'))
```

Vue 3.5 adds lazy hydration strategies for SSR (`hydrateOnVisible`), but
praxis's Tauri dashboard is client-side only — regular async components are
sufficient.

### `v-once` for static content

```vue
<header v-once>
  <h1>{{ appTitle }}</h1>  <!-- rendered once, never updated -->
</header>
```

---

## What to avoid

| Anti-pattern | Do instead |
|---|---|
| Options API (`export default { data() {} }`) | `<script setup>` |
| `this.xxx` | Direct refs / composables |
| `any` in TypeScript | `unknown` + type narrowing |
| `async/await` | Promise chains (`.then().catch().finally()`) |
| `var` / `let` (when avoidable) | `const` |
| `.push()` / `.splice()` / `.sort()` | Spread `[...arr, item]` / `toSorted()` |
| `console.log` in committed code | Remove before commit |
| Destructuring Pinia store directly | `storeToRefs()` for state/getters |
| `ref(null)` + `ref="name"` for template refs | `useTemplateRef('name')` (3.5) |
| `withDefaults(defineProps<...>(), {...})` | Reactive props destructure (3.5) |
| Manual `props` + `emit` for `v-model` | `defineModel()` |
| Hardcoded hex colors | `var(--clr-*)` design tokens |
| `forEach` for transformations | `map` / `filter` / `reduce` |
