<script setup lang="ts">
import { getToolName, isReasoningUIPart, isTextUIPart, isToolUIPart } from 'ai'
import { isPartStreaming, isToolStreaming } from '@nuxt/ui/utils/ai'
import type { UIMessage } from '~/types/chat'

/**
 * Renders one message's parts. Split out of the page because the same
 * rendering is needed wherever messages appear, and because the page was
 * getting long enough to hide the important bit — the part-type branching.
 */
defineProps<{ message: UIMessage }>()
</script>

<template>
  <template
    v-for="(part, index) in message.parts"
    :key="`${message.id}-${part.type}-${index}`"
  >
    <UChatReasoning
      v-if="isReasoningUIPart(part)"
      :text="part.text"
      :streaming="isPartStreaming(part)"
    />

    <ChatToolCall
      v-else-if="isToolUIPart(part)"
      :part="part"
      :tool-name="getToolName(part)"
      :streaming="isToolStreaming(part)"
    />

    <template v-else-if="isTextUIPart(part)">
      <Comark
        v-if="message.role === 'assistant'"
        :markdown="part.text"
        :streaming="isPartStreaming(part)"
        class="*:first:mt-0 *:last:mb-0"
      />
      <p
        v-else
        class="whitespace-pre-wrap"
      >
        {{ part.text }}
      </p>
    </template>
  </template>
</template>
