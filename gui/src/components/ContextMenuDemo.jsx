import React from 'react';
import { useContextMenuRegistration } from '../hooks/useContextMenu';
import { useI18n } from '../i18n';

/**
 * 上下文菜单管理组件
 * 提供注册/卸载右键菜单的功能
 * 文件处理逻辑已移至 App.jsx 和 SendTab 中统一管理
 */
export function ContextMenuDemo() {
  const { t } = useI18n();
  
  const {
    registerContextMenu,
    unregisterContextMenu,
    isRegistering,
    isUnregistering,
    registrationStatus,
    registrationError
  } = useContextMenuRegistration();

  return (
    <div className="space-y-4">
      {/* 上下文菜单注册/卸载 */}
      <div className="bg-white rounded-xl border border-gray-200 p-4 space-y-3">
        <h3 className="text-lg font-semibold text-gray-800">{t('contextMenu.title') || '右键菜单'}</h3>
        <p className="text-gray-600 text-sm">
          {t('contextMenu.description') || '注册 Windows/Linux 的右键菜单选项。右键点击文件或文件夹时可以看到 QuickShare 的多项功能。'}
        </p>
        
        <div className="flex items-center gap-3">
          <button
            onClick={registerContextMenu}
            disabled={isRegistering || isUnregistering}
            className="flex items-center gap-2 px-4 py-2 bg-primary-500 text-white rounded-lg hover:bg-primary-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium"
          >
            {isRegistering ? (
              <>
                <svg className="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                </svg>
                {t('contextMenu.registering') || '注册中...'}
              </>
            ) : (
              t('contextMenu.register') || '注册右键菜单'
            )}
          </button>
          
          <button
            onClick={unregisterContextMenu}
            disabled={isRegistering || isUnregistering}
            className="flex items-center gap-2 px-4 py-2 bg-red-50 text-red-600 border border-red-200 rounded-lg hover:bg-red-100 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium"
          >
            {isUnregistering ? (
              <>
                <svg className="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                </svg>
                {t('contextMenu.unregistering') || '卸载中...'}
              </>
            ) : (
              t('contextMenu.unregister') || '卸载右键菜单'
            )}
          </button>
        </div>
        
        {registrationStatus && (
          <div className="p-3 bg-green-50 border border-green-200 rounded-lg text-green-700 text-sm">
            ✓ {registrationStatus}
          </div>
        )}
        
        {registrationError && (
          <div className="p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm">
            ✗ {registrationError}
          </div>
        )}
      </div>

      {/* 菜单功能说明 */}
      <div className="bg-gray-50 rounded-xl p-4">
        <h3 className="text-sm font-semibold text-gray-600 mb-3">{t('contextMenu.menuOptions') || '右键菜单选项'}</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div className="p-3 bg-white rounded-lg border border-gray-100">
            <h4 className="font-medium text-blue-600 text-sm">{t('contextMenu.send') || '发送到 QuickShare'}</h4>
            <p className="text-xs text-gray-500 mt-1">{t('contextMenu.sendDesc') || '直接发送文件到 QuickShare，等待选择目标设备'}</p>
          </div>
          <div className="p-3 bg-white rounded-lg border border-gray-100">
            <h4 className="font-medium text-green-600 text-sm">{t('contextMenu.sendMulti') || '发送到多设备...'}</h4>
            <p className="text-xs text-gray-500 mt-1">{t('contextMenu.sendMultiDesc') || '可以选择多个设备同时发送文件'}</p>
          </div>
          <div className="p-3 bg-white rounded-lg border border-gray-100">
            <h4 className="font-medium text-purple-600 text-sm">{t('contextMenu.sendRecent') || '发送到最近设备'}</h4>
            <p className="text-xs text-gray-500 mt-1">{t('contextMenu.sendRecentDesc') || '快速发送到上次使用过的设备'}</p>
          </div>
          <div className="p-3 bg-white rounded-lg border border-gray-100">
            <h4 className="font-medium text-orange-600 text-sm">{t('contextMenu.queue') || '添加到发送队列'}</h4>
            <p className="text-xs text-gray-500 mt-1">{t('contextMenu.queueDesc') || '将文件添加到队列，稍后批量发送'}</p>
          </div>
        </div>
      </div>

      {/* 使用说明 */}
      <div className="bg-amber-50 border border-amber-200 rounded-xl p-4">
        <h3 className="text-sm font-semibold text-amber-700 mb-2">{t('contextMenu.instructions') || '使用说明'}</h3>
        <ul className="list-disc list-inside space-y-1 text-xs text-amber-700">
          <li>{t('contextMenu.instruction1') || '点击"注册右键菜单"按钮注册 QuickShare 的右键菜单选项'}</li>
          <li>{t('contextMenu.instruction2') || '在文件管理器右键点击文件或文件夹，选择 QuickShare 子菜单'}</li>
          <li>{t('contextMenu.instruction3') || '选择不同的功能选项来发送文件'}</li>
          <li>{t('contextMenu.instruction4') || '不再需要时，点击"卸载右键菜单"按钮移除'}</li>
        </ul>
      </div>
    </div>
  );
}
