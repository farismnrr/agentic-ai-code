<script setup lang="ts">
import type { AccordionItem } from '@nuxt/ui'

definePageMeta({ layout: 'landing' })

const title = 'AI Code — chat with models, connected to your tools'
const description
  = 'A chat interface with streaming replies, MCP tool calling with explicit approval, and multiple models. Securely authenticated and backed by a real database.'

useSeoMeta({ title, description, ogTitle: title, ogDescription: description })

const features = [
  {
    title: 'Streaming replies',
    description:
      'Tokens appear as they arrive, with reasoning shown in a collapsible block. Stop mid-answer, or regenerate.',
    icon: 'i-lucide-zap'
  },
  {
    title: 'MCP tools, with consent',
    description:
      'Connect Model Context Protocol servers and pick which tools a conversation may use. Every call asks before it runs.',
    icon: 'i-lucide-blocks'
  },
  {
    title: 'Choose your model',
    description:
      'Switch between models per conversation, or set a default. Temperature and custom instructions live in settings.',
    icon: 'i-lucide-sparkles'
  },
  {
    title: 'Securely persisted',
    description:
      'Conversations, tool configurations, and settings are securely stored and synced to your account.',
    icon: 'i-lucide-shield-check'
  }
]

const plans = [
  {
    title: 'Free',
    description: 'Enough to see whether it fits.',
    price: '$0',
    billingCycle: '/month',
    features: ['200 messages a month', '1 MCP server', 'Haiku 4.5', 'Community support'],
    button: { label: 'Get started', to: '/register', color: 'neutral' as const, variant: 'subtle' as const }
  },
  {
    title: 'Pro',
    description: 'For daily work.',
    price: '$20',
    billingCycle: '/month',
    badge: 'Popular',
    highlight: true,
    features: [
      'Unlimited messages',
      'Unlimited MCP servers',
      'Every model, including Opus 5',
      'Tool approval policies',
      'Priority support'
    ],
    button: { label: 'Get started', to: '/register' }
  },
  {
    title: 'Team',
    description: 'Shared tools and billing.',
    price: '$40',
    billingCycle: '/seat/month',
    features: [
      'Everything in Pro',
      'Shared MCP server registry',
      'Org-wide approval rules',
      'Audit log',
      'SSO'
    ],
    button: { label: 'Contact sales', to: '/register', color: 'neutral' as const, variant: 'subtle' as const }
  }
]

const testimonials = [
  {
    quote:
      'The tool approval dialog is the part I did not know I needed. Seeing the arguments before a call runs changed how much I trust it.',
    user: { name: 'Rina Ashari', description: 'Staff Engineer', avatar: { alt: 'Rina Ashari' } }
  },
  {
    quote:
      'Switching models mid-conversation without losing the thread sounds small. It is not, when you are debugging something long.',
    user: { name: 'Daniel Okoro', description: 'Platform Lead', avatar: { alt: 'Daniel Okoro' } }
  },
  {
    quote:
      'We pointed it at our internal MCP servers and it just picked up the tools. No glue code.',
    user: { name: 'Mei Tanaka', description: 'Developer Experience', avatar: { alt: 'Mei Tanaka' } }
  }
]

const faq: AccordionItem[] = [
  {
    label: 'Does this actually talk to a model?',
    content:
      'Yes. It connects to real AI models via the 9Router API with streaming responses.'
  },
  {
    label: 'What is MCP?',
    content:
      'Model Context Protocol: a standard way to expose tools to a model. A server advertises tools, the model asks to call one, and here you approve or deny it before anything runs.'
  },
  {
    label: 'Is my data stored anywhere?',
    content:
      'Yes. Your account, settings, conversations, and MCP configurations are securely persisted in a real database.'
  },
  {
    label: 'Can I use my own MCP servers?',
    content:
      'Yes. You can add them in settings and they show up in the tool picker. Note: in this phase, the backend securely stores your configuration but does not yet execute tool calls during chat.'
  },
  {
    label: 'Is the pricing real?',
    content:
      'No. There is nothing to buy. The plans are here to show what the page would look like.'
  }
]
</script>

<template>
  <div class="relative overflow-hidden">
    <!-- Dynamic Ambient Background -->
    <div class="pointer-events-none absolute inset-0 z-[-1] overflow-hidden">
      <!-- Grid pattern -->
      <div class="absolute inset-0 bg-[linear-gradient(to_right,#80808012_1px,transparent_1px),linear-gradient(to_bottom,#80808012_1px,transparent_1px)] bg-[size:24px_24px] [mask-image:radial-gradient(ellipse_60%_50%_at_50%_0%,#000_70%,transparent_100%)]" />

      <!-- Glowing blobs -->
      <div
        class="absolute top-0 left-1/2 -translate-x-1/2 w-full max-w-3xl h-[400px] bg-primary/20 blur-[120px] rounded-[100%] animate-pulse"
        style="animation-duration: 4s;"
      />
      <div
        class="absolute top-[10%] right-[-10%] w-[500px] h-[500px] bg-blue-500/10 blur-[120px] rounded-[100%] animate-pulse"
        style="animation-duration: 7s; animation-delay: 1s;"
      />
      <div
        class="absolute top-[20%] left-[-10%] w-[600px] h-[600px] bg-purple-500/10 blur-[120px] rounded-[100%] animate-pulse"
        style="animation-duration: 5s; animation-delay: 2s;"
      />
    </div>

    <UPageHero
      description="Streaming replies, MCP tool calling that asks before it acts, and a model picker that does not lose your thread. Powered by a real Postgres backend."
      :links="[
        { label: 'Get started', to: '/register', size: 'xl', trailingIcon: 'i-lucide-arrow-right' },
        { label: 'Sign in', to: '/login', size: 'xl', color: 'neutral', variant: 'subtle' }
      ]"
    >
      <template #title>
        <span class="text-transparent bg-clip-text bg-gradient-to-r from-primary to-blue-500">
          Chat with models, <br class="hidden sm:block"> connected to your tools
        </span>
      </template>

      <div class="relative mx-auto mt-12 max-w-2xl group">
        <div class="absolute -inset-1 rounded-xl bg-gradient-to-r from-primary to-blue-500 opacity-20 blur-lg transition-opacity duration-1000 group-hover:opacity-40" />
        <LandingHeroDemo class="relative" />
      </div>
    </UPageHero>

    <RevealSection>
      <UPageSection
        id="features"
        title="What it does"
        description="The parts that make a chat interface usable rather than merely present."
      >
        <UPageGrid>
          <UPageCard
            v-for="feature in features"
            :key="feature.title"
            v-bind="feature"
            spotlight
          />
        </UPageGrid>
      </UPageSection>
    </RevealSection>

    <RevealSection>
      <UPageSection
        id="pricing"
        title="Pricing"
        description="Illustrative. Nothing here charges anyone."
      >
        <UPricingPlans>
          <UPricingPlan
            v-for="plan in plans"
            :key="plan.title"
            v-bind="plan"
          />
        </UPricingPlans>
      </UPageSection>
    </RevealSection>

    <RevealSection>
      <UPageSection
        title="What people say"
        description="Invented, like the pricing."
      >
        <UPageColumns>
          <UPageCard
            v-for="testimonial in testimonials"
            :key="testimonial.user.name"
            variant="subtle"
            :description="testimonial.quote"
          >
            <template #footer>
              <UUser
                v-bind="testimonial.user"
                size="sm"
              />
            </template>
          </UPageCard>
        </UPageColumns>
      </UPageSection>
    </RevealSection>

    <RevealSection>
      <UPageSection
        id="faq"
        title="Questions"
      >
        <UAccordion
          :items="faq"
          class="mx-auto max-w-2xl"
        />
      </UPageSection>
    </RevealSection>

    <RevealSection>
      <UPageCTA
        title="Try it"
        description="Sign up for an account to start chatting with models and tools."
        variant="subtle"
        :links="[
          { label: 'Get started', to: '/register', size: 'lg', trailingIcon: 'i-lucide-arrow-right' }
        ]"
      />
    </RevealSection>
  </div>
</template>
