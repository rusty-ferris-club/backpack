import React from 'react'
import { SimpleGrid, Text, VStack } from '@chakra-ui/react'

const FEATURES = [
  {
    title: '👩‍💻 Use ordinary repos',
    description:
      'Generate from project, subfolders, branches, tags - or any parts of repos you like',
  },
  {
    title: '😎 Choose from a menu',
    description:
      'Shortcuts - create a personal or team list of your projects with global and local shortcuts',
  },
  {
    title: '🪄 Replace content',
    description:
      'Variable replacements - replace variables in content and path (like cookiecutter)',
  },
  {
    title: '🤖 Automate boring tasks',
    description:
      'Automated setup and preparation steps. Run installs, compilation, or test automatically after a clone',
  },
  {
    title: '🙋 Ask nicely',
    description:
      'Interactive inputs - define steps to take inputs and select options in YAML while generating a new project',
  },
  {
    title: '🚀️ Optimized for content',
    description:
      'Fast & efficient - no history or .git folder, local caching by default, git and tar.gz download',
  },
]

const Feature = ({ title, description }) => (
  <VStack
    flexDirection="column"
    spacing={4}
    alignItems={['center', 'center', 'flex-start']}
  >
    <Text as="h3" fontSize="2xl" fontWeight={600}>
      {title}
    </Text>
    <Text letterSpacing="-0.03em" fontWeight={400}>
      {description}
    </Text>
  </VStack>
)

export const Features = ({ features = FEATURES }) => {
  return (
    <SimpleGrid columns={[1, 1, 3]} spacing="60px">
      {features.map((f) => (
        <Feature key={f.title} {...f} />
      ))}
    </SimpleGrid>
  )
}
