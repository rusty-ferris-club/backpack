import { extendTheme } from '@chakra-ui/react'
import { mode } from '@chakra-ui/theme-tools'

// 2. Call `extendTheme` and pass your custom values
const theme = extendTheme({
  fonts: {
    heading: "'Jost', sans-serif",
    body: "'Inter var', sans-serif",
  },
  styles: {
    global: {
      body: {
        fontFeatureSettings: '"cv02", "cv03", "cv04", "cv11"',
        background: 't_background',
        color: 't_text',
      },
      h1: { letterSpacing: '-0.03em', color: 't_text' },
      h2: { letterSpacing: '-0.03em', color: 't_text' },
      h3: { letterSpacing: '-0.02em', color: 't_text' },
      svg: {
        display: 'inline',
      },
    },
  },
  sizes: {
    max: '100%',
    container: {
      xl2: '1440px',
    },
  },
  colors: {
    brand: {
      50: '#FFF0F0',
      100: '#FBB1D8',
      200: '#FA99CC',
      300: '#F87CBD',
      400: '#F655A9',
      500: '#F545A1',
      600: '#F3208E',
      700: '#DF0C7A',
      800: '#C20A6A',
      900: '#A5095A',
    },
    charcoal: '#111',
    canvas: '#ffffff',
    outline: 'blue',
    text: '#444',
    textLighter: '#555',
    textLightest: '#666',
    textDarker: '#222',
    textDocs: '#555',
  },
  semanticTokens: {
    colors: {
      t_border_color: {
        default: 'blackAlpha.50',
        _dark: 'whiteAlpha.300',
      },
      t_strong: {
        default: 'textDarker',
        _dark: 'whiteAlpha.900',
      },
      t_text_docs: {
        default: 'textDocs',
        _dark: 'whiteAlpha.800',
      },
      t_text: {
        default: 'text',
        _dark: 'whiteAlpha.800',
      },
      t_weak: {
        default: 'textLighter',
        _dark: 'whiteAlpha.800',
      },
      t_weakest: {
        default: 'textLightest',
        _dark: 'whiteAlpha.800',
      },
      t_background: {
        default: 'canvas',
        _dark: 'transparent',
      },
      t_background_docs: {
        default: 'canvas',
        _dark: 'charcoal',
      },
      t_background_article: {
        default: 'white',
        _dark: 'charcoal',
      },
      ink: {
        default: 'text',
        _dark: 'canvas',
      },
      inverseInk: {
        default: 'canvas',
        _dark: 'text',
      },
    },
  },
  shadows: {
    outline: '0 0 0 3px #FB309Aca',
  },
  components: {
    Link: {
      baseStyle: {
        color: 'brand.500',
        transition: 'color 200ms',
        _hover: {
          color: 'brand.500',
        },
        _focus: {
          boxShadow: 'none',
        },
      },
    },
    Code: {
      variants: {
        installer: (props) => ({
          border: 'none',
          background: 'none',
          color: mode('black', 'white')(props),
          fontSize: 16,
        }),
      },
    },
    Button: {
      sizes: {
        xl: {
          h: '60px',
          minW: 16,
          fontSize: 'md',
          px: 7,
        },
      },
      variants: {
        outline: {
          borderWidth: '2px',
          _hover: {
            textDecoration: 'none',
          },
        },
        'clipboard-copy': (props) => ({
          bg: 'ink',
          color: 'inverseInk',
          _focus: {
            shadow: 'none',
          },
          _hover: {
            bg: mode('blackAlpha.700', 'whiteAlpha.900')(props),
          },
          _active: {
            bg: mode('blackAlpha.800', 'whiteAlpha.800')(props),
          },
        }),
      },
    },
  },
})
console.log('theme', theme)
export default theme
