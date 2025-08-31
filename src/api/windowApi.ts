import { invoke } from '@tauri-apps/api/core';

export interface WindowInfo {
  width: number;
  height: number;
  x: number;
  y: number;
  maximized: boolean;
  minimized: boolean;
}

export interface DisplayInfo {
  width: number;
  height: number;
  x: number;
  y: number;
  scaleFactor: number;
  isPrimary: boolean;
}

export interface SystemInfo {
  displays: DisplayInfo[];
  primaryDisplayIndex: number;
}

export const windowApi = {
  // ウィンドウ情報を取得
  async getWindowInfo(): Promise<WindowInfo> {
    try {
      const info = await invoke<WindowInfo>('get_window_info');
      return info;
    } catch (error: unknown) {
      console.error('Failed to get window info:', error);
      throw error;
    }
  },

  // ウィンドウ情報を設定
  async setWindowInfo(info: WindowInfo): Promise<void> {
    try {
      await invoke('set_window_info', { info });
    } catch (error: unknown) {
      console.error('Failed to set window info:', error);
      throw error;
    }
  },

  // システム情報を取得
  async getSystemInfo(): Promise<SystemInfo> {
    try {
      const info = await invoke<SystemInfo>('get_system_info');
      console.log('Debug: Received system info from Rust:', info);
      return info;
    } catch (error: unknown) {
      console.error('Failed to get system info:', error);
      throw error;
    }
  },

    // ディスプレイ情報が一致するかチェック
  async isDisplayConfigurationSame(savedSystemInfo: SystemInfo): Promise<boolean> {
    try {
      const currentSystemInfo = await this.getSystemInfo();

      console.log('Debug: Comparing display configurations:');
      console.log('Current system info:', {
        displaysCount: currentSystemInfo.displays.length,
        primaryIndex: currentSystemInfo.primaryDisplayIndex,
        displays: currentSystemInfo.displays.map((d, i) => ({
          index: i,
          width: d.width,
          height: d.height,
          x: d.x,
          y: d.y,
          scaleFactor: d.scaleFactor,
          isPrimary: d.isPrimary
        }))
      });
      console.log('Saved system info:', {
        displaysCount: savedSystemInfo.displays.length,
        primaryIndex: savedSystemInfo.primaryDisplayIndex,
        displays: savedSystemInfo.displays.map((d, i) => ({
          index: i,
          width: d.width,
          height: d.height,
          x: d.x,
          y: d.y,
          scaleFactor: d.scaleFactor,
          isPrimary: d.isPrimary
        }))
      });

      // ディスプレイ数が同じかチェック
      if (currentSystemInfo.displays.length !== savedSystemInfo.displays.length) {
        console.log('Display count mismatch:', currentSystemInfo.displays.length, 'vs', savedSystemInfo.displays.length);
        return false;
      }

      // 各ディスプレイの情報を比較
      for (let i = 0; i < currentSystemInfo.displays.length; i++) {
        const current = currentSystemInfo.displays[i];
        const saved = savedSystemInfo.displays[i];

        console.log(`Comparing display ${i}:`, {
          current: { width: current.width, height: current.height, scaleFactor: current.scaleFactor, isPrimary: current.isPrimary },
          saved: { width: saved.width, height: saved.height, scaleFactor: saved.scaleFactor, isPrimary: saved.isPrimary },
          match: current.width === saved.width &&
                 current.height === saved.height &&
                 current.scaleFactor === saved.scaleFactor &&
                 current.isPrimary === saved.isPrimary
        });

        if (
          current.width !== saved.width ||
          current.height !== saved.height ||
          current.scaleFactor !== saved.scaleFactor ||
          current.isPrimary !== saved.isPrimary
        ) {
          console.log(`Display ${i} configuration mismatch`);
          return false;
        }
      }

      console.log('Display configurations match!');
      return true;
    } catch (error: unknown) {
      console.error('Failed to check display configuration:', error);
      return false;
    }
  },

      // ウィンドウ情報が有効かチェック
  isWindowInfoValid(info: WindowInfo, systemInfo: SystemInfo): boolean {
    console.log('Debug: Checking window info validity:', {
      windowInfo: info,
      systemInfo: {
        displaysCount: systemInfo.displays.length,
        primaryIndex: systemInfo.primaryDisplayIndex
      }
    });

    // ディスプレイ情報が存在するかチェック
    if (!systemInfo.displays || systemInfo.displays.length === 0) {
      console.warn('No displays found in system info');
      return false;
    }

    // プライマリディスプレイインデックスが有効かチェック
    if (systemInfo.primaryDisplayIndex >= systemInfo.displays.length) {
      console.warn('Invalid primary display index:', systemInfo.primaryDisplayIndex);
      return false;
    }

    const primaryDisplay = systemInfo.displays[systemInfo.primaryDisplayIndex];

    // プライマリディスプレイの情報が完全かチェック
    if (!primaryDisplay || typeof primaryDisplay.x !== 'number' || typeof primaryDisplay.y !== 'number' ||
        typeof primaryDisplay.width !== 'number' || typeof primaryDisplay.height !== 'number') {
      console.warn('Invalid primary display info:', primaryDisplay);
      console.log('System info details:', {
        displaysCount: systemInfo.displays.length,
        primaryIndex: systemInfo.primaryDisplayIndex,
        displays: systemInfo.displays,
        isValidIndex: systemInfo.primaryDisplayIndex < systemInfo.displays.length,
        primaryDisplay: systemInfo.displays[systemInfo.primaryDisplayIndex],
        allDisplays: systemInfo.displays.map((d, i) => ({
          index: i,
          width: d.width,
          height: d.height,
          x: d.x,
          y: d.y,
          isPrimary: d.isPrimary
        }))
      });
      return false;
    }

    console.log('Debug: Primary display info:', {
      x: primaryDisplay.x,
      y: primaryDisplay.y,
      width: primaryDisplay.width,
      height: primaryDisplay.height
    });

    // ウィンドウがディスプレイ内に収まっているかチェック
    if (info.x < primaryDisplay.x || info.y < primaryDisplay.y) {
      console.log('Window position outside display bounds (top-left)');
      return false;
    }

    if (info.x + info.width > primaryDisplay.x + primaryDisplay.width ||
        info.y + info.height > primaryDisplay.y + primaryDisplay.height) {
      console.log('Window position outside display bounds (bottom-right)');
      return false;
    }

    // 最小サイズチェック
    if (info.width < 800 || info.height < 600) {
      console.log('Window size too small:', info.width, 'x', info.height);
      return false;
    }

    console.log('Window info is valid!');
    return true;
  }
};
