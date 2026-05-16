import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: {
        translation: {
          app: { name: 'envSwitch', version: 'v0.1.0' },
          nav: { versions: 'Versions', services: 'Services', status: 'Status', logs: 'Logs', doctor: 'Doctor', settings: 'Settings' },
          common: { refresh: 'Refresh', save: 'Save', cancel: 'Cancel', cover: 'Cover', uncover: 'Uncover', start: 'Start', stop: 'Stop', install: 'Install', search: 'Search', active: 'active', noVersions: 'No versions installed', selectModule: 'Select a module' },
          service: { running: 'Running', stopped: 'Stopped', pid: 'PID', port: 'Port', dataDir: 'Data' },
          settings: { title: 'Settings', subtitle: 'Configure envSwitch behavior', language: 'Language', languageDesc: 'Switch between English and Chinese', proxy: 'Proxy', proxyDesc: 'HTTP/HTTPS proxy for downloads', brewSource: 'Homebrew Source', brewSourceDesc: 'Custom Homebrew registry mirror' },
          doctor: { title: 'Doctor', subtitle: 'Diagnose setup issues', allGood: 'All checks passed', issuesFound: 'issues found' },
          toast: { covered: 'covered', uncovered: 'uncovered', started: 'started', stopped: 'stopped', error: 'Error' }
        }
      },
      zh: {
        translation: {
          app: { name: 'envSwitch', version: 'v0.1.0' },
          nav: { versions: '版本', services: '服务', status: '状态', logs: '日志', doctor: '诊断', settings: '设置' },
          common: { refresh: '刷新', save: '保存', cancel: '取消', cover: '覆盖', uncover: '取消覆盖', start: '启动', stop: '停止', install: '安装', search: '搜索', active: '已激活', noVersions: '未安装版本', selectModule: '选择一个模块' },
          service: { running: '运行中', stopped: '已停止', pid: '进程', port: '端口', dataDir: '数据' },
          settings: { title: '设置', subtitle: '配置 envSwitch 行为', language: '语言', languageDesc: '切换中文和英文界面', proxy: '代理', proxyDesc: '下载使用的 HTTP/HTTPS 代理', brewSource: 'Homebrew 源', brewSourceDesc: '自定义 Homebrew 镜像源' },
          doctor: { title: '诊断', subtitle: '诊断配置问题', allGood: '所有检查通过', issuesFound: '个问题' },
          toast: { covered: '已覆盖', uncovered: '已取消覆盖', started: '已启动', stopped: '已停止', error: '错误' }
        }
      }
    },
    fallbackLng: 'en',
    detection: { order: ['localStorage', 'navigator'], caches: ['localStorage'] },
  });

export default i18n;
