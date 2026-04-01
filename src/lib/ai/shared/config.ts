import type { AppConfig, FeaturesConfig } from '$lib/stores/config';

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

export function updateNestedPath<T>(
  obj: T,
  path: string[],
  updater: (value: unknown) => unknown
): T {
  const [key, ...rest] = path;
  if (rest.length === 0) {
    return { ...obj, [key]: updater((obj as Record<string, unknown>)[key]) };
  }
  return {
    ...obj,
    [key]: updateNestedPath(
      (obj as Record<string, unknown>)[key] as Record<string, unknown>,
      rest,
      updater
    )
  } as T;
}
