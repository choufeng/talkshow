import type { AppConfig, FeaturesConfig } from './types';

/**
 * 不可变更新嵌套配置并返回新对象
 * 用法: updateFeature(config, 'transcription', t => ({ ...t, model: 'new-model' }))
 */
export function updateFeature<K extends keyof FeaturesConfig>(
  config: AppConfig,
  featureKey: K,
  updater: (feature: FeaturesConfig[K]) => FeaturesConfig[K]
): AppConfig {
  return {
    ...config,
    features: {
      ...config.features,
      [featureKey]: updater(config.features[featureKey])
    }
  };
}

/**
 * 深度更新嵌套对象路径
 * 用法: updateNestedPath(config, ['ai', 'providers'], providers => [...providers, newProvider])
 */
export function updateNestedPath<T extends Record<string, unknown>>(
  obj: T,
  path: string[],
  updater: (value: unknown) => unknown
): T {
  const [key, ...rest] = path;
  if (rest.length === 0) {
    return { ...obj, [key]: updater(obj[key as keyof T]) };
  }
  return {
    ...obj,
    [key]: updateNestedPath(
      obj[key as keyof T] as Record<string, unknown>,
      rest,
      updater
    )
  } as T;
}
