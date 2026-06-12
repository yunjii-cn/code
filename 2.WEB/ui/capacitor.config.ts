import type { CapacitorConfig } from '@capacitor/cli'

const config: CapacitorConfig = {
  appId: 'com.yunjii.code',
  appName: '云集智能编程工作站',
  webDir: 'dist',
  androidScheme: 'https',
  server: {
    androidScheme: 'https',
  },
  plugins: {
    SplashScreen: {
      launchShowDuration: 2000,
      backgroundColor: '#0d0d0d',
      showSpinner: false,
      androidScaleType: 'CENTER_CROP',
    },
  },
}

export default config
