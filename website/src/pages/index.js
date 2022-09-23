import React from 'react'
import Layout from '@theme/Layout'
import { Hero } from '../components/hero'
import { Features } from '../components/features'
import { Section } from '../components/section'

export default function Home() {
  return (
    <Layout>
      <Section>
        <Hero />
      </Section>
      <Section>
        <Features />
      </Section>
    </Layout>
  )
}
