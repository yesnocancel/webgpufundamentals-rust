Title: WebGPU 可选特性与限制
Description: 可选特性
TOC: 可选特性与限制

WebGPU 有一系列可选特性和限制。让我们来看看如何检查它们以及如何请求它们。

当你请求一个适配器时

```js
const adapter = await navigator.gpu?.requestAdapter();
```

适配器会包含 `adapter.limits` 上的限制列表和 `adapter.features` 上的特性名称数组。例如

```js
const adapter = await navigator.gpu?.requestAdapter();
console.log(adapter.limits.maxColorAttachments);
```

可能会在控制台打印 `8`，表示该适配器最多支持 8 个颜色附件。

下面列出了所有的限制，包括你的默认适配器的限制以及最低要求限制。

<div class="webgpu_center data-table limits" data-diagram="limits"></div>

最低限制是指所有支持 WebGPU 的设备都可以保证具备的限制。

还有一个可选特性列表。例如，你可以这样查看它们

```js
const adapter = await navigator.gpu?.requestAdapter();
console.log(adapter.features);
```

可能会打印类似 `["texture-compression-astc", "texture-compression-bc"]` 的内容，告诉你这些特性在请求后可用。

下面是你的默认适配器上可用特性的列表。

<div class="webgpu_center data-table features" data-diagram="features"></div>

> 注意：你可以在 [webgpureport.org](https://webgpureport.org) 查看你系统适配器的所有特性和限制。

## 请求限制和特性

默认情况下，当你请求设备时，你会获得最低限制（上表右侧列）并且不会获得任何可选特性。这样做的目的是，只要你保持在最低限制之内，你的应用就能在所有支持 WebGPU 的设备上运行。

但是，根据适配器上列出的可用限制和特性，你可以在调用 `requestDevice` 时通过将你想要的限制作为 `requiredLimits` 传入，以及将你想要的特性作为 `requiredFeatures` 传入来请求它们。例如

```js
const k1Gig = 1024 * 1024 * 1024;
const adapter = await navigator.gpu?.requestAdapter();
const device = adapter?.requestDevice({
  requiredLimits: { maxBufferSize: k1Gig },
  requiredFeatures: [ 'float32-filterable' ],
});
```

上面我们请求能够使用最大 1GB 的缓冲区，以及能够使用可过滤的 float32 纹理（例如 `'rgba32float'` 配合 `minFilter` 设置为 `'linear'`，默认情况下只能使用 `'nearest'`）

如果其中任何一个请求无法满足，`requestDevice` 将会失败（拒绝 promise）。

## 不要请求所有内容

可能会很想请求所有的限制和特性，然后检查你需要哪些。

例如：

```js
function objLikeToObj(src) {
  const dst = {};
  for (const key in src) {
    dst[key] = src[key];
  }
  return dst;
}

//
// 不好！！！ ❌
//
async function main() {
  const adapter = await navigator?.gpu.requestAdapter();
  const device = await adapter?.requestDevice({
    requiredLimits: objLikeToObj(adapter.limits),
    requiredFeatures: adapter.features,
  });
  if (!device) {
    fail('need webgpu');
    return;
  }

  const canUse128KUniformsBuffers = device.limits.maxUniformBufferBindingSize >= 128 * 1024;
  const canStoreToBGRA8Unorm = device.features.has('bgra8unorm-storage');
  const canIndirectFirstInstance = device.features.has('indirect-first-instance');
}
```

这看起来是一种简单明了的方式来检查限制和特性[^objliketoobj]。这种模式的问题在于，你可能会在不知不觉中超过限制。例如，假设你创建了一个 `'rgba32float'` 纹理并使用 `'linear'` 过滤。

在你的台式机上它会神奇地正常工作，因为你恰好启用了它。

[^objliketoobj]: 这个 `objLikeToObj` 是什么？为什么需要它？这是 Web 规范的一个晦涩问题。规范将 `requiredLimits` 列为 `record<DOMString, GPUSize64>`。Web IDL 规范说，在将对象转换为 `record<DOMString, GPUSize64>` 时，只复制实际上是对象自身属性的属性。适配器上的 `limits` 对象被列为一个 `interface`。在那里看起来像是属性的东西实际上不是属性，而是存在于对象原型上的 getter，它们实际上不是对象的属性。因此，在转换为 `record<DOMString, GPUSize64>` 时它们不会被复制，所以你必须自己复制它们。

在用户的手机上，你的程序会神秘地失败，因为 `'float32-filterable'` 特性不存在，而你恰好在不知不觉中使用了它。

或者你可能分配了一个大于最低 `maxBufferSize` 的缓冲区，同样没有意识到自己超过了限制。你发布后，大量用户无法运行你的页面。

## 推荐的方式来请求特性和限制

推荐使用特性和限制的方式是，只请求你绝对需要的东西。

例如

```js
  const adapter = await navigator?.gpu.requestAdapter();

  const canUse128KUniformsBuffers = adapter?.limits.maxUniformBufferBindingSize >= 128 * 1024;
  const canStoreToBGRA8Unorm = adapter?.features.has('bgra8unorm-storage');
  const canIndirectFirstInstance = adapter?.features.has('indirect-first-instance');

  // 如果我们绝对需要这些特性中的一个或多个，那么如果它们不可用现在就失败
  if (!canUse128kUniformBuffers) {
    alert('抱歉，你的设备可能太旧或性能不足');
    return;
  }

  // 请求我们需要的可用特性和限制
  const device = adapter?.requestDevice({
    requiredFeatures: [
      ...(canStorageBGRA8Unorm ? ['bgra8unorm'] : []),
      ...(canIndirectFirstInstance) ? ['indirect-first-instance']),
    ],
    requiredLimit: [
      maxUniformBufferBindingSize: 128 * 1024,
    ]
  });
```

这样做的好处是，如果你恰好请求了一个大于 128k 的 Uniform 缓冲区，你会收到错误。同样，如果你尝试使用你没有请求的特性，你也会收到错误。然后你可以自主决定是想增加你需要的限制（因此会拒绝在更多设备上运行），还是想保持限制不变，或者是否想重构代码以便在特性或限制可用或不可用时做不同的事情。

<!-- keep this at the bottom of the article -->
<link rel="stylesheet" href="webgpu-limits-and-features.css">
<script type="module" src="webgpu-limits-and-features.js"></script>
