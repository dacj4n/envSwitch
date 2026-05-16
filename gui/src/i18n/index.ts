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
          common: { refresh: 'Refresh', save: 'Save', cancel: 'Cancel', cover: 'Cover', uncover: 'Uncover', start: 'Start', stop: 'Stop', install: 'Install', search: 'Search', fetch: 'Fetch', active: 'ACTIVE', noVersions: 'No versions installed', selectModule: 'Select a module', available: 'Available', startAll: 'Start All', stopAll: 'Stop All', installed: 'installed', availableCount: 'available', loading: 'Loading...', noLogs: 'No log entries — the service may not have been started yet', noVersionFound: 'No installed version found', commands: 'commands', done: 'Done' },
          service: { running: 'Running', stopped: 'Stopped', pid: 'PID', port: 'Port', dataDir: 'Data', cliRef: 'CLI Quick Reference', logViewer: 'Log Viewer', manageSubtitle: 'Manage database services', logsSubtitle: 'Real service logs from running instances' },
          install: { title: 'Installing...', starting: 'Starting...', running: 'Running', complete: 'Complete', failed: 'Failed', cancelled: 'Cancelled', cancelling: 'Cancelling...', waiting: 'Waiting for job to begin...', cancel: 'Cancel' },
          versions: { subtitle: 'Manage installed SDK versions', clickFetch: 'click Fetch to load available versions' },
          status: { subtitle: 'Active version covers', envStatus: 'Environment Status', active: 'active', noActiveCovers: 'No active covers', notCovered: 'not covered', inactive: 'inactive', shimsHint: 'Shims directory: ~/.envswitch/shims — ensure it\'s in your $PATH' },
          doctor: { subtitle: 'Diagnose setup issues', runningChecks: 'Running checks...', allOk: 'All systems OK', checksPassed: 'checks passed', issuesFound: 'issues found', warnings: 'warnings', platformDetected: 'Platform detected', modulesLoaded: 'Modules loaded', shimsDir: 'Shims directory', brewAvailable: 'Homebrew available', brewFound: 'brew found', brewNotFound: 'brew not found', countModules: 'modules' },
          settings: { title: 'Settings', subtitle: 'Configure envSwitch behavior', language: 'Language', languageDesc: 'Switch between English and Chinese', proxy: 'Proxy', proxyDesc: 'HTTP/HTTPS proxy for downloads', brewSource: 'Homebrew Source', brewSourceDesc: 'Custom Homebrew registry mirror', cliExamples: 'CLI Examples', saved: 'Saved', proxySaved: 'Proxy saved — takes effect on next install', copied: 'Copied', switchedEn: 'Switched to English', switchedZh: '已切换为中文' },
          toast: { covered: 'covered', uncovered: 'uncovered', started: 'started', stopped: 'stopped', error: 'Error' },
          cli: { searchJdk: 'Search available JDK versions', installJdk: 'Install JDK 21', coverJdk: 'Activate JDK 21 (shim switch)', coverGo: 'Activate Go 1.25.10', status: 'Show current cover stack', uncoverAll: 'Restore all system defaults', list: 'List installed versions', doctor: 'Diagnose setup issues', startMysql: 'Start MySQL service', cdHook: 'Enable auto-switch on cd' }
        }
      },
      zh: {
        translation: {
          app: { name: 'envSwitch', version: 'v0.1.0' },
          nav: { versions: '版本', services: '服务', status: '状态', logs: '日志', doctor: '诊断', settings: '设置' },
          common: { refresh: '刷新', save: '保存', cancel: '取消', cover: '覆盖', uncover: '取消覆盖', start: '启动', stop: '停止', install: '安装', search: '搜索', fetch: '获取', active: '已激活', noVersions: '未安装版本', selectModule: '选择一个模块', available: '可用版本', startAll: '全部启动', stopAll: '全部停止', installed: '已安装', availableCount: '可用', loading: '加载中...', noLogs: '暂无日志 — 服务可能尚未启动', noVersionFound: '未找到已安装版本', commands: '命令', done: '完成' },
          service: { running: '运行中', stopped: '已停止', pid: '进程', port: '端口', dataDir: '数据', cliRef: 'CLI 快速参考', logViewer: '日志查看器', manageSubtitle: '管理数据库服务', logsSubtitle: '运行中的服务日志' },
          install: { title: '正在安装...', starting: '启动中...', running: '运行中', complete: '完成', failed: '失败', cancelled: '已取消', cancelling: '取消中...', waiting: '等待任务开始...', cancel: '取消' },
          versions: { subtitle: '管理已安装的 SDK 版本', clickFetch: '点击 Fetch 加载可用版本' },
          status: { subtitle: '版本覆盖状态', envStatus: '环境状态', active: '已激活', noActiveCovers: '无已激活版本', notCovered: '未覆盖', inactive: '未激活', shimsHint: 'Shims 目录: ~/.envswitch/shims — 请确保它在 $PATH 中' },
          doctor: { subtitle: '诊断配置问题', runningChecks: '检查中...', allOk: '所有系统正常', checksPassed: '项通过', issuesFound: '个问题', warnings: '个警告', platformDetected: '平台检测', modulesLoaded: '模块加载', shimsDir: 'Shims 目录', brewAvailable: 'Homebrew 可用', brewFound: '已发现 brew', brewNotFound: '未找到 brew', countModules: '个模块' },
          settings: { title: '设置', subtitle: '配置 envSwitch 行为', language: '语言', languageDesc: '切换中文和英文界面', proxy: '代理', proxyDesc: '下载使用的 HTTP/HTTPS 代理', brewSource: 'Homebrew 源', brewSourceDesc: '自定义 Homebrew 镜像源', cliExamples: 'CLI 示例', saved: '已保存', proxySaved: '代理已保存 — 下次安装时生效', copied: '已复制', switchedEn: 'Switched to English', switchedZh: '已切换为中文' },
          toast: { covered: '已覆盖', uncovered: '已取消覆盖', started: '已启动', stopped: '已停止', error: '错误' },
          cli: { searchJdk: '搜索可用的 JDK 版本', installJdk: '安装 JDK 21', coverJdk: '激活 JDK 21 (shim 切换)', coverGo: '激活 Go 1.25.10', status: '显示当前覆盖状态', uncoverAll: '恢复所有系统默认', list: '列出已安装版本', doctor: '诊断配置问题', startMysql: '启动 MySQL 服务', cdHook: '启用 cd 自动切换' }
        }
      }
    },
    fallbackLng: 'en',
    detection: { order: ['localStorage', 'navigator'], caches: ['localStorage'] },
  });

export default i18n;
