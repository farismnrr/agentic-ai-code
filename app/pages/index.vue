<script setup lang="ts">
import type { AccordionItem } from '@nuxt/ui'

definePageMeta({ layout: 'landing' })

const title = 'AI Code — chat with models, connected to your tools'
const description
  = 'A chat interface with streaming replies, MCP tool calling with explicit approval, and multiple models. Prototype: no backend.'

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
    title: 'Nothing leaves the page',
    description:
      'This build has no backend. State lives in the browser and resets on reload — it is a prototype, honestly.',
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
      'No. This build has no backend — replies come from fixtures and stream through the real AI SDK plumbing. Swapping in a live endpoint is a one-line change to the transport.'
  },
  {
    label: 'What is MCP?',
    content:
      'Model Context Protocol: a standard way to expose tools to a model. A server advertises tools, the model asks to call one, and here you approve or deny it before anything runs.'
  },
  {
    label: 'Is my data stored anywhere?',
    content:
      'Only your session, so a refresh does not sign you out. Conversations, settings and server lists live in memory and reset when you reload the page.'
  },
  {
    label: 'Can I use my own MCP servers?',
    content:
      'You can add them in settings and they show up in the tool picker. In this prototype nothing connects — the list is dummy data.'
  },
  {
    label: 'Is the pricing real?',
    content:
      'No. There is nothing to buy. The plans are here to show what the page would look like.'
  }
]
</script>

<template>
  <div>
    <UPageHero
      title="Chat with models, connected to your tools"
      description="Streaming replies, MCP tool calling that asks before it acts, and a model picker that does not lose your thread. This is a prototype — no backend behind it."
      :links="[
        { label: 'Get started', to: '/register', size: 'xl', trailingIcon: 'i-lucide-arrow-right' },
        { label: 'Sign in', to: '/login', size: 'xl', color: 'neutral', variant: 'subtle' }
      ]"
    >
      <LandingHeroDemo class="mt-12" />
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
        description="Any email and password gets you in. Nothing is stored."
        variant="subtle"
        :links="[
          { label: 'Get started', to: '/register', size: 'lg', trailingIcon: 'i-lucide-arrow-right' }
        ]"
      />
    </RevealSection>
  </div>
</template>
