Title: WebGPU Utilsとwgpu-matrix
Description: WebGPU用のユーティリティと数学
TOC: WebGPU Utilsと数学

<div class="warn">この記事はGemini Code Assistによって自動翻訳されました。翻訳に問題がある場合は、お手数ですが<a href="https://github.com/webgpu/webgpufundamentals/pulls">こちら</a>からPull Requestを送信してください。</div>

> ## この記事から得られるべきこと
>
> WebGPUの使用は非常に冗長です。非常に冗長なので、いくつかのヘルパーを使用すると、より高いレベルの概念に集中できるため、理解しやすくなります。
>
> たとえば、数学を学んでいるとします。先生は「平均」とは何か、そしていくつかの数値のセットの平均を計算する方法を教えます。それを教えた後、彼らは他のことに移り、「ここで平均を計算します」と言うだけです。たとえば、
>
> > 標準偏差を計算するには
> > 
> > * すべてのデータの平均を計算します
> > * データセットの各数値について、その数値と平均の差を計算します。
> > * 各差を見つけたら、それを2乗します。
> > * 2乗差の平均の平方根を取ります
>
> 彼らは平均の計算方法を再説明しません。あなたはすでにそれを学んでおり、彼らはあなたがすでに学んだことを参照するだけです。
>
> 同様に、WebGPUでは、WGSLでユニフォーム用の構造体を作成するという概念があります。次に、1つ以上のユニフォームバッファを作成し、それらのバッファに`TypedArrays`を使用してデータを入力します。これについては、このサイトの最初の20〜30の記事と[メモリレイアウトに関する記事](webgpu-memory-layout.html)で詳しく説明しました。
>
> しかし、ある時点で、「ユニフォームを設定する」と言うだけで、これらの詳細を扱うコードを理解するのが難しくなります。そして、あなたは以前に「ユニフォームを設定する」とは、「さまざまなデータへのオフセットを計算し、そのデータを設定できるように型付き配列ビューを作成する。そして後で、レンダリングする前に、それを設定してGPUに値をアップロードする」ことを意味することを学びました。
>
> そのため、このサイトで使用されているライブラリを恐れないでください。その機能のほとんどすべては、サイトの最初の記事で詳しく説明されています。以下にいくつかの詳細を示します。

このサイトの多くの例では、2つのライブラリを使用しています。

## wgpu-matrix

1つ目は[wgpu-matrix](https://github.com/greggman/wgpu-matrix)です。wgpu-matrixは、[行列演算に関する記事](webgpu-matrix-math.html)から[遠近投影に関する記事](webgpu-perspective-projection.html)、および[ライティングに関する記事](webgpu-lighting-directional.html)で記述したのと同じ関数のコレクションです。

ここでは特別なことは何も起こっていません。数学関数のいずれかがどのように機能するかを知りたい場合は、上記の記事を読むことができます。

## webgpu-utils

2つ目は[webgpu-utils](https://github.com/greggman/webgpu-utils)です。

WebGPU Utilsは、さまざまな記事で記述した他の便利な関数のコレクションです。たとえば、関数

* `numMipLevels`
* `loadImageBitmap`
* `copySourceToTexture`
* `createTextureFromSource`
* `createTextureFromImage`
* `generateMips`

これらはすべて、[テクスチャへの画像のインポートに関する記事](webgpu-importing-textures.html)で作成しました。

また、

* `copySourcesToTexture`
* `createTextureFromSources`
* `generateMips`

[キューブマップに関する記事](webgpu-cube-maps.html)から。その記事では、複数のレイヤーを処理するように`generateMips`を更新しました。

そして、[透明度とブレンディングに関する記事](webgpu-transparency.html)で`premultipliedAlpha`のサポートを追加した方法が含まれています。

このライブラリには、

* `createTextureFromImages`

[環境マップに関する記事](webgpu-environment-maps.html)から。

### `makeShaderDataDefinitions`と`makeStructuredView`

これらの2つの関数は、[メモリレイアウトに関する記事](webgpu-memory-layout.html)で簡単に触れました。

[基本的な記事](webgpu-fundamentals.html)のすべて、および[行列演算に関する記事](webgpu-matrix-math.html)と[ライティングに関する記事](webgpu-lighting-directional.html)で見たように、WGSLで構造体を作成する場合、通常、ユニフォームバッファまたはストレージバッファを作成し、何らかの方法でデータを入力する必要があります。

特に、ライティングに関する記事でこれを見ることができます。この構造体がありました。

```wgsl
struct Uniforms {
  matrix: mat4x4f,
  color: vec4f,
  lightDirection: vec3f,
};
```

次に、これに変更されました。

```wgsl
struct Uniforms {
  world: mat4x4f,
  worldViewProjection: mat4x4f,
  color: vec4f,
  lightDirection: vec3f,
};
```

次にこれ

```wgsl
struct Uniforms {
  normalMatrix: mat3x3f,
  worldViewProjection: mat4x4f,
  color: vec4f,
  lightDirection: vec3f,
};
```

そしてこれ

```wgsl
struct Uniforms {
  normalMatrix: mat3x3f,
  worldViewProjection: mat4x4f,
  world: mat4x4f,
  color: vec4f,
  lightPosition: vec3f,
};
```

これに続いて

```wgsl
struct Uniforms {
  normalMatrix: mat3x3f,
  worldViewProjection: mat4x4f,
  world: mat4x4f,
  color: vec4f,
  lightWorldPosition: vec3f,
  viewWorldPosition: vec3f,
};
```

そしてこれ

```wgsl
struct Uniforms {
  normalMatrix: mat3x3f,
  worldViewProjection: mat4x4f,
  world: mat4x4f,
  color: vec4f,
  lightWorldPosition: vec3f,
  viewWorldPosition: vec3f,
  shininess: f32,
};
```

そしてこれ

```wgsl
struct Uniforms {
  normalMatrix: mat3x3f,
  worldViewProjection: mat4x4f,
  world: mat4x4f,
  color: vec4f,
  lightWorldPosition: vec3f,
  viewWorldPosition: vec3f,
  shininess: f32,
  lightDirection: vec3f,
  limit: f32,
};
```

そしてこれ

```wgsl
struct Uniforms {
  normalMatrix: mat3x3f,
  worldViewProjection: mat4x4f,
  world: mat4x4f,
  color: vec4f,
  lightWorldPosition: vec3f,
  viewWorldPosition: vec3f,
  shininess: f32,
  lightDirection: vec3f,
  innerLimit: f32,
  outerLimit: f32,
};
```

これらの変更を行うたびに、ビューを設定するコードに入り、非常に多くのものを編集する必要がありました。何をする必要があったかを説明するために、進行状況を次に示します。

[指向性ライティングに関する記事](webgpu-lighting-directional.html)でここから始めました。

```js
  // 行列 + 色 + 光の方向
  const uniformBufferSize = (16 + 4 + 4) * 4;
  const uniformBuffer = device.createBuffer({
    label: 'uniforms',
    size: uniformBufferSize,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const uniformValues = new Float32Array(uniformBufferSize / 4);

  // float32インデックスでのさまざまなユニフォーム値へのオフセット
  const kMatrixOffset = 0;
  const kColorOffset = 16;
  const kLightDirectionOffset = 20;

  const matrixValue = uniformValues.subarray(kMatrixOffset, kMatrixOffset + 16);
  const colorValue = uniformValues.subarray(kColorOffset, kColorOffset + 4);
  const lightDirectionValue =
      uniformValues.subarray(kLightDirectionOffset, kLightDirectionOffset + 3);
```

次にこれ

```js
-  const uniformBufferSize = (16 + 4 + 4) * 4;
+  const uniformBufferSize = (16 + 16 + 4 + 4) * 4;
  const uniformBuffer = device.createBuffer({
    label: 'uniforms',
    size: uniformBufferSize,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const uniformValues = new Float32Array(uniformBufferSize / 4);

  // float32インデックスでのさまざまなユニフォーム値へのオフセット
-  const kMatrixOffset = 0;
-  const kColorOffset = 16;
-  const kLightDirectionOffset = 20;
+  const kWorldOffset = 0;
+  const kWorldViewProjectionOffset = 16;
+  const kColorOffset = 32;
+  const kLightDirectionOffset = 36;

-  const matrixValue = uniformValues.subarray(kMatrixOffset, kMatrixOffset + 16);
+  const worldValue = uniformValues.subarray(kWorldOffset, kWorldOffset + 16);
+  const worldViewProjectionValue = uniformValues.subarray(
      kWorldViewProjectionOffset, kWorldViewProjectionOffset + 16);
  const colorValue = uniformValues.subarray(kColorOffset, kColorOffset + 4);
  const lightDirectionValue =
      uniformValues.subarray(kLightDirectionOffset, kLightDirectionOffset + 3);
```

次にこれ

```js
-  const uniformBufferSize = (16 + 16 + 4 + 4) * 4;
+  const uniformBufferSize = (12 + 16 + 4 + 4) * 4;
  const uniformBuffer = device.createBuffer({
    label: 'uniforms',
    size: uniformBufferSize,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const uniformValues = new Float32Array(uniformBufferSize / 4);

  // float32インデックスでのさまざまなユニフォーム値へのオフセット
-  const kWorldOffset = 0;
-  const kWorldViewProjectionOffset = 16;
-  const kColorOffset = 32;
-  const kLightDirectionOffset = 36;
+  const kNormalMatrixOffset = 0;
+  const kWorldViewProjectionOffset = 12;
+  const kColorOffset = 28;
+  const kLightDirectionOffset = 32;

-  const worldValue = uniformValues.subarray(kWorldOffset, kWorldOffset + 16);
+  const normalMatrixValue = uniformValues.subarray(
+      kNormalMatrixOffset, kNormalMatrixOffset + 12);
  const worldViewProjectionValue = uniformValues.subarray(
      kWorldViewProjectionOffset, kWorldViewProjectionOffset + 16);
  const colorValue = uniformValues.subarray(kColorOffset, kColorOffset + 4);
  const lightDirectionValue =
      uniformValues.subarray(kLightDirectionOffset, kLightDirectionOffset + 3);
```

そしてこれ

```js
-  const uniformBufferSize = (12 + 16 + 4 + 4) * 4;
+  const uniformBufferSize = (12 + 16 + 16 + 4 + 4) * 4;
  const uniformBuffer = device.createBuffer({
    label: 'uniforms',
    size: uniformBufferSize,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const uniformValues = new Float32Array(uniformBufferSize / 4);

  // float32インデックスでのさまざまなユニフォーム値へのオフセット
  const kNormalMatrixOffset = 0;
  const kWorldViewProjectionOffset = 12;
-  const kColorOffset = 28;
-  const kLightDirectionOffset = 32;
+  const kWorldOffset = 28;
+  const kColorOffset = 44;
+  const kLightPositionOffset = 48;

  const normalMatrixValue = uniformValues.subarray(
      kNormalMatrixOffset, kNormalMatrixOffset + 12);
  const worldViewProjectionValue = uniformValues.subarray(
      kWorldViewProjectionOffset, kWorldViewProjectionOffset + 16);
+  const worldValue = uniformValues.subarray(
+      kWorldOffset, kWorldOffset + 16);
  const colorValue = uniformValues.subarray(kColorOffset, kColorOffset + 4);
-  const lightDirectionValue =
-      uniformValues.subarray(kLightDirectionOffset, kLightDirectionOffset + 3);
+  const lightPositionValue =
+      uniformValues.subarray(kLightPositionOffset, kLightPositionOffset + 3);
```

これに続いて

```js
-  const uniformBufferSize = (12 + 16 + 16 + 4 + 4) * 4;
+  const uniformBufferSize = (12 + 16 + 16 + 4 + 4 + 4) * 4;
  const uniformBuffer = device.createBuffer({
    label: 'uniforms',
    size: uniformBufferSize,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const uniformValues = new Float32Array(uniformBufferSize / 4);

  // float32インデックスでのさまざまなユニフォーム値へのオフセット
  const kNormalMatrixOffset = 0;
  const kWorldViewProjectionOffset = 12;
  const kWorldOffset = 28;
  const kColorOffset = 44;
  const kLightPositionOffset = 48;
+  const kViewWorldPositionOffset = 52;

  const normalMatrixValue = uniformValues.subarray(
      kNormalMatrixOffset, kNormalMatrixOffset + 12);
  const worldViewProjectionValue = uniformValues.subarray(
      kWorldViewProjectionOffset, kWorldViewProjectionOffset + 16);
  const worldValue = uniformValues.subarray(
      kWorldOffset, kWorldOffset + 16);
  const colorValue = uniformValues.subarray(kColorOffset, kColorOffset + 4);
  const lightPositionValue = uniformValues.subarray(
      kLightPositionOffset, kLightPositionOffset + 3);
+  const viewWorldPositionValue = uniformValues.subarray(
+      kViewWorldPositionOffset, kViewWorldPositionOffset + 3);
```

そしてこれ

```js
  const kNormalMatrixOffset = 0;
  const kWorldViewProjectionOffset = 12;
  const kWorldOffset = 28;
  const kColorOffset = 44;
  const kLightWorldPositionOffset = 48;
  const kViewWorldPositionOffset = 52;
+  const kShininessOffset = 55;

  const normalMatrixValue = uniformValues.subarray(
      kNormalMatrixOffset, kNormalMatrixOffset + 12);
  const worldViewProjectionValue = uniformValues.subarray(
      kWorldViewProjectionOffset, kWorldViewProjectionOffset + 16);
  const worldValue = uniformValues.subarray(
      kWorldOffset, kWorldOffset + 16);
  const colorValue = uniformValues.subarray(kColorOffset, kColorOffset + 4);
  const lightWorldPositionValue = uniformValues.subarray(
      kLightWorldPositionOffset, kLightWorldPositionOffset + 3);
  const viewWorldPositionValue = uniformValues.subarray(
      kViewWorldPositionOffset, kViewWorldPositionOffset + 3);
+  const shininessValue = uniformValues.subarray(
+      kShininessOffset, kShininessOffset + 1);
```

そしてこれ

```js
-  const uniformBufferSize = (12 + 16 + 16 + 4 + 4 + 4) * 4;
+  const uniformBufferSize = (12 + 16 + 16 + 4 + 4 + 4 + 4) * 4;
  const uniformBuffer = device.createBuffer({
    label: 'uniforms',
    size: uniformBufferSize,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const uniformValues = new Float32Array(uniformBufferSize / 4);

  // float32インデックスでのさまざまなユニフォーム値へのオフセット
  const kNormalMatrixOffset = 0;
  const kWorldViewProjectionOffset = 12;
  const kWorldOffset = 28;
  const kColorOffset = 44;
  const kLightWorldPositionOffset = 48;
  const kViewWorldPositionOffset = 52;
  const kShininessOffset = 55;
+  const kLightDirectionOffset = 56;
+  const kLimitOffset = 59;

  const normalMatrixValue = uniformValues.subarray(
      kNormalMatrixOffset, kNormalMatrixOffset + 12);
  const worldViewProjectionValue = uniformValues.subarray(
      kWorldViewProjectionOffset, kWorldViewProjectionOffset + 16);
  const worldValue = uniformValues.subarray(
      kWorldOffset, kWorldOffset + 16);
  const colorValue = uniformValues.subarray(kColorOffset, kColorOffset + 4);
  const lightWorldPositionValue = uniformValues.subarray(
      kLightWorldPositionOffset, kLightWorldPositionOffset + 3);
  const viewWorldPositionValue = uniformValues.subarray(
      kViewWorldPositionOffset, kViewWorldPositionOffset + 3);
  const shininessValue = uniformValues.subarray(
      kShininessOffset, kShininessOffset + 1);
+  const lightDirectionValue = uniformValues.subarray(
+      kLightDirectionOffset, kLightDirectionOffset + 3);
+  const limitValue = uniformValues.subarray(
+      kLimitOffset, kLimitOffset + 1);
```

そして最後に、[スポットライトに関する記事](webgpu-lighting-spot.html)の最後からこれです。

```js
-  const uniformBufferSize = (12 + 16 + 16 + 4 + 4 + 4 + 4) * 4;
+  const uniformBufferSize = (12 + 16 + 16 + 4 + 4 + 4 + 4 + 4) * 4;
  const uniformBuffer = device.createBuffer({
    label: 'uniforms',
    size: uniformBufferSize,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const uniformValues = new Float32Array(uniformBufferSize / 4);

  // float32インデックスでのさまざまなユニフォーム値へのオフセット
  const kNormalMatrixOffset = 0;
  const kWorldViewProjectionOffset = 12;
  const kWorldOffset = 28;
  const kColorOffset = 44;
  const kLightWorldPositionOffset = 48;
  const kViewWorldPositionOffset = 52;
  const kShininessOffset = 55;
  const kLightDirectionOffset = 56;
-  const kLimitOffset = 59;
+  const kInnerLimitOffset = 59;
+  const kOuterLimitOffset = 60;

  const normalMatrixValue = uniformValues.subarray(
      kNormalMatrixOffset, kNormalMatrixOffset + 12);
  const worldViewProjectionValue = uniformValues.subarray(
      kWorldViewProjectionOffset, kWorldViewProjectionOffset + 16);
  const worldValue = uniformValues.subarray(
      kWorldOffset, kWorldOffset + 16);
  const colorValue = uniformValues.subarray(kColorOffset, kColorOffset + 4);
  const lightWorldPositionValue = uniformValues.subarray(
      kLightWorldPositionOffset, kLightWorldPositionOffset + 3);
  const viewWorldPositionValue = uniformValues.subarray(
      kViewWorldPositionOffset, kViewWorldPositionOffset + 3);
  const shininessValue = uniformValues.subarray(
      kShininessOffset, kShininessOffset + 1);
  const lightDirectionValue = uniformValues.subarray(
      kLightDirectionOffset, kLightDirectionOffset + 3);
-  const limitValue = uniformValues.subarray(
-      kLimitOffset, kLimitOffset + 1);
+  const innerLimitValue = uniformValues.subarray(
+      kInnerLimitOffset, kInnerLimitOffset + 1);
+  const outerLimitValue = uniformValues.subarray(
+      kOuterLimitOffset, kOuterLimitOffset + 1);
```

**この冗長性は、記事の要点から注意をそらしている**ことを願っています！私たちが本当に言いたかったのは、「WGSL構造体をこれに変更し、描画する前に値を設定する」ということだけですが、代わりに、**例ごとに**40行以上のコード変更を示しています。

`makeShaderDataDefinitions`と`makeStructuredView`を使用すると、上記のすべてのJavaScriptをこれらの7行に変更できます。

```js
const defs = makeShaderDataDefinitions(code);
const uni = makeStructuredView(defs.uniforms.uni);

const uniformBuffer = device.createBuffer({
  size: uni.arrayBuffer.byteLength,
  usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
});
```

以上です。サンプル間で、必要に応じて構造体を変更しますが、これらの2つの関数は、これらのオフセットとビューをすべて作成します。

最後の例の構造体を取ると、

```wgsl
struct Uniforms {
  normalMatrix: mat3x3f,
  worldViewProjection: mat4x4f,
  world: mat4x4f,
  color: vec4f,
  lightWorldPosition: vec3f,
  viewWorldPosition: vec3f,
  shininess: f32,
  lightDirection: vec3f,
  innerLimit: f32,
  outerLimit: f32,
};

*@group(0) @binding(0) var<uniform> uni: Uniforms;
```

これらの2行

```js
const defs = makeShaderDataDefinitions(code);
const uni = makeStructuredView(defs.uniforms.uni);
```

`WGSL`で定義したユニフォームバインディングである`uni`の「構造化ビュー」を作成します。

事実上、これらの行はこれを作成します。

```js
const arrayBuffer = new ArrayBuffer(256);
const uni = {
  arrayBuffer,
  set: function(data) { /* helper */ },
  views: {
    normalMatrix: new Float32Array(arrayBuffer, 0, 12),
    worldViewProjection: new Float32Array(arrayBuffer, 48, 16),
    world: new Float32Array(arrayBuffer, 112, 16),
    color: new Float32Array(arrayBuffer, 176, 4),
    lightWorldPosition: new Float32Array(arrayBuffer, 192, 3),
    viewWorldPosition: new Float32Array(arrayBuffer, 208, 3),
    shininess: new Float32Array(arrayBuffer, 220, 1),
    lightDirection: new Float32Array(arrayBuffer, 224, 3),
    innerLimit: new Float32Array(arrayBuffer, 236, 1),
    outerLimit: new Float32Array(arrayBuffer, 240, 1),
  },
};
```

`makeShaderDataDefinitions`が実際にWGSLを解析して、これらのビューを作成するのに十分なデータを抽出するという事実を除いて、ここに魔法はありません。

上記の記事では、値を設定するために次のようなコードがありました。

```js
    const aspect = canvas.clientWidth / canvas.clientHeight;
    const projection = mat4.perspective(
        degToRad(60),
        aspect,
        1,      // zNear
        2000,   // zFar
    );

    const eye = [100, 150, 200];
    const target = [0, 35, 0];
    const up = [0, 1, 0];

    // ビュー行列を計算します
    const viewMatrix = mat4.lookAt(eye, target, up);

    // ビュー行列と射影行列を組み合わせます
    const viewProjectionMatrix = mat4.multiply(projection, viewMatrix);

    // ワールド行列を計算します
    const world = mat4.rotationY(settings.rotation, worldValue);

    // ビュー射影行列とワールド行列を組み合わせます
    mat4.multiply(viewProjectionMatrix, world, worldViewProjectionValue);

    // 逆行列と転置行列をnormalMatrix値に変換します
    mat3.fromMat4(mat4.transpose(mat4.inverse(world)), normalMatrixValue);

    colorValue.set([0.2, 1, 0.2, 1]);  // green
    lightWorldPositionValue.set([-10, 30, 100]);
    viewWorldPositionValue.set(eye);
    shininessValue[0] = settings.shininess;
    innerLimitValue[0] = Math.cos(settings.innerLimit);
    outerLimitValue[0] = Math.cos(settings.outerLimit);

    // ほとんどのスポットライトの例のように平面がないので、
    // スポットライトをFに向けましょう
    {
        const mat = mat4.aim(
            lightWorldPositionValue,
            [
              target[0] + settings.aimOffsetX,
              target[1] + settings.aimOffsetY,
              0,
            ],
            up);
        // 行列からzAxisを取得します
        // lookAtは-Z軸を見下ろすため、それを否定します
        lightDirectionValue.set(mat.slice(8, 11));
    }

    // ユニフォーム値をユニフォームバッファにアップロードします
    device.queue.writeBuffer(uniformBuffer, 0, uniformValues);
```

そのコードはこれに変更できます。

```js
+    // 同じ既存の名前を使用してビューをプルアウトします。
+    const {
+      world: worldValue,
+      worldViewProjection: worldViewProjectionValue,
+      normalMatrix: normalMatrixValue,
+      color: colorValue,
+      lightWorldPosition: lightWorldPositionValue,
+      lightDirection: lightDirectionValue,
+      viewWorldPosition: viewWorldPositionValue,
+      shininess: shininessValue,
+      innerLimit: innerLimitValue,
+      outerLimit: outerLimitValue,
+    } = uni.views;

    const aspect = canvas.clientWidth / canvas.clientHeight;
    const projection = mat4.perspective(
        degToRad(60),
        aspect,
        1,      // zNear
        2000,   // zFar
    );

    const eye = [100, 150, 200];
    const target = [0, 35, 0];
    const up = [0, 1, 0];

    // ビュー行列を計算します
    const viewMatrix = mat4.lookAt(eye, target, up);

    // ビュー行列と射影行列を組み合わせます
    const viewProjectionMatrix = mat4.multiply(projection, viewMatrix);

    // ワールド行列を計算します
    const world = mat4.rotationY(settings.rotation, worldValue);

    // ビュー射影行列とワールド行列を組み合わせます
    mat4.multiply(viewProjectionMatrix, world, worldViewProjectionValue);

    // 逆行列と転置行列をnormalMatrix値に変換します
    mat3.fromMat4(mat4.transpose(mat4.inverse(world)), normalMatrixValue);

    colorValue.set([0.2, 1, 0.2, 1]);  // green
    lightWorldPositionValue.set([-10, 30, 100]);
    viewWorldPositionValue.set(eye);
    shininessValue[0] = settings.shininess;
    innerLimitValue[0] = Math.cos(settings.innerLimit);
    outerLimitValue[0] = Math.cos(settings.outerLimit);

    // ほとんどのスポットライトの例のように平面がないので、
    // スポットライトをFに向けましょう
    {
        const mat = mat4.aim(
            lightWorldPositionValue,
            [
              target[0] + settings.aimOffsetX,
              target[1] + settings.aimOffsetY,
              0,
            ],
            up);
        // 行列からzAxisを取得します
        // lookAtは-Z軸を見下ろすため、それを否定します
        lightDirectionValue.set(mat.slice(8, 11));
    }

    // ユニフォーム値をユニフォームバッファにアップロードします
-    device.queue.writeBuffer(uniformBuffer, 0, uniformValues);
+    device.queue.writeBuffer(uniformBuffer, 0, uni.arrayBuffer);
```

または、ビューを直接使用できます。

```js
-    // 同じ既存の名前を使用してビューをプルアウトします。
-    const {
-      world: worldValue,
-      worldViewProjection: worldViewProjectionValue,
-      normalMatrix: normalMatrixValue,
-      color: colorValue,
-      lightWorldPosition: lightWorldPositionValue,
-      lightDirection: lightDirectionValue,
-      viewWorldPosition: viewWorldPositionValue,
-      shininess: shininessValue,
-      innerLimit: innerLimitValue,
-      outerLimit: outerLimitValue,
-    } = uni.views;
+   const { views } = uni;

    const aspect = canvas.clientWidth / canvas.clientHeight;
    const projection = mat4.perspective(
        degToRad(60),
        aspect,
        1,      // zNear
        2000,   // zFar
    );

    const eye = [100, 150, 200];
    const target = [0, 35, 0];
    const up = [0, 1, 0];

    // ビュー行列を計算します
    const viewMatrix = mat4.lookAt(eye, target, up);

    // ビュー行列と射影行列を組み合わせます
    const viewProjectionMatrix = mat4.multiply(projection, viewMatrix);

    // ワールド行列を計算します
-    const world = mat4.rotationY(settings.rotation, worldValue);
+    const world = mat4.rotationY(settings.rotation, views.world);

    // ビュー射影行列とワールド行列を組み合わせます
-    mat4.multiply(viewProjectionMatrix, world, worldViewProjectionValue);
+    mat4.multiply(viewProjectionMatrix, world, views.worldViewProjection);

    // 逆行列と転置行列をnormalMatrix値に変換します
-    mat3.fromMat4(mat4.transpose(mat4.inverse(world)), normalMatrixValue);
+    mat3.fromMat4(mat4.transpose(mat4.inverse(world)), views.normalMatrix);

-    views.color.set([0.2, 1, 0.2, 1]);  // green
-    views.lightWorldPosition.set([-10, 30, 100]);
-    views.viewWorldPosition.set(eye);
-    views.shininess[0] = settings.shininess;
-    views.innerLimit[0] = Math.cos(settings.innerLimit);
-    views.outerLimit[0] = Math.cos(settings.outerLimit);
+    uni.set({
+      color: [0.2, 1, 0.2, 1],  // green
+      lightWorldPosition: [-10, 30, 100],
+      viewWorldPosition: eye,
+      shininess: settings.shininess,
+      innerLimit: settings.innerLimit,
+      outerLimit: settings.outerLimit,
+    });

    // ほとんどのスポットライトの例のように平面がないので、
    // スポットライトをFに向けましょう
    {
        const mat = mat4.aim(
-            lightWorldPositionValue,
+            views.lightWorldPosition,
            [
              target[0] + settings.aimOffsetX,
              target[1] + settings.aimOffsetY,
              0,
            ],
            up);
        // 行列からzAxisを取得します
        // lookAtは-Z軸を見下ろすため、それを否定します
-        views.lightDirection.set(mat.slice(8, 11));
+        uni.set({ lightDirectionValue: mat.slice(8, 11) });
    }

    // ユニフォーム値をユニフォームバッファにアップロードします
    device.queue.writeBuffer(uniformBuffer, 0, uni.arrayBuffer);
```

`set`関数は、上記で示したユースケースでは、かなり単純であると想像できます。

これは機能します。

```js
const arrayBuffer = new ArrayBuffer(256);
const views = {
  normalMatrix: new Float32Array(arrayBuffer, 0, 12),
  worldViewProjection: new Float32Array(arrayBuffer, 48, 16),
  world: new Float32Array(arrayBuffer, 112, 16),
  color: new Float32Array(arrayBuffer, 176, 4),
  lightWorldPosition: new Float32Array(arrayBuffer, 192, 3),
  viewWorldPosition: new Float32Array(arrayBuffer, 208, 3),
  shininess: new Float32Array(arrayBuffer, 220, 1),
  lightDirection: new Float32Array(arrayBuffer, 224, 3),
  innerLimit: new Float32Array(arrayBuffer, 236, 1),
  outerLimit: new Float32Array(arrayBuffer, 240, 1),
};
const uni = {
  arrayBuffer,
  set: function(data) {
    // 過度に単純化
    for (const [key, value] of Object.entries(data)) {
      const view = views[key];
      if (view) {
        view.set(typeof value === 'number' ? [value] : value);
      }
    }
  },
};
```

実際の`set`の実装は、ネストされた構造体と配列を処理するために、わずかに複雑です。詳細については、ソースを参照してください。
これが「set」のコードです：[リンク](https://github.com/greggman/webgpu-utils/blob/cb61348691718e22f877e0011673f84d456927b6/src/buffer-views.ts#L291)
そして、これが呼び出す関数のコードです：[リンク](https://github.com/greggman/webgpu-utils/blob/cb61348691718e22f877e0011673f84d456927b6/src/buffer-views.ts#L386)

上記の例が、それが魔法ではないことを明確にすることを願っています。これらの単純な関数は、WebGPUの使用をはるかに面倒でなくし、物事をはるかに単純に説明できます。「ユニフォーム値を設定する」と言うだけで、オフセットの計算、ビューの作成などの面倒な作業を150回目に見せる必要はありません。

## 頂点バッファと属性

もう1つ、面倒を減らすことができる場所は、頂点バッファと属性の設定です。問題は通常、頂点位置、頂点法線、頂点テクスチャ座標などのデータが必要なことです。それらを別々の配列で作成できます。これは簡単です。

```js
const positions = [];
const normals = [];
const texcoords = [];

for(each vertex) {
  ...
  position.push(x, y, z);
  normals.push(nx, ny, nz);
  texcoord.push(u, v);
}
```

これで、3つのバッファと3つの属性セットが必要になるという追加の複雑さが生じます。

```js
  const pipeline = device.createRenderPipeline({
    vertex: {
      module: shaderModule,
*      buffers: [
*        // position
*        {
*          arrayStride: 3 * 4, // 3 floats, 4 bytes each
*          attributes: [
*            {shaderLocation: 0, offset: 0, format: 'float32x3'},
*          ],
*        },
*        // normals
*        {
*          arrayStride: 3 * 4, // 3 floats, 4 bytes each
*          attributes: [
*            {shaderLocation: 1, offset: 0, format: 'float32x3'},
*          ],
*        },
*        // texcoords
*        {
*          arrayStride: 2 * 4, // 2 floats, 4 bytes each
*          attributes: [
*            {shaderLocation: 2, offset: 0, format: 'float32x2',},
*          ],
*        },
*      ],
    },

...

  function createBuffer(device, values, usage) {
    const data = new Float32Array(values);
    const buffer = device.createBuffer({
      size: data.byteLength,
      usage,
      mappedAtCreation: true,
    });
    const dst = new data.constructor(buffer.getMappedRange());
    dst.set(data);
    buffer.unmap();
    return buffer;
  }

  const positionBuffer = createBuffer(device, positions, GPUBufferUsage.VERTEX);
  const normalBuffer = createBuffer(device, normals, GPUBufferUsage.VERTEX);
  const texcoordBuffer = createBuffer(device, texcoords, GPUBufferUsage.VERTEX);

```

もっと面倒です。😮‍💨

または、それらをインターリーブしようとすることもできます。これは簡単かもしれないし、そうでないかもしれません。すべてが同じ型、たとえばすべて32ビット浮動小数点値である場合は、次のようなことができます。

```js
const vertexData = [];

for (each vertex) {
  ...
  vertexData.push(
      x, y, z,
      nx, ny, nz,
      u, v);
}
```

しかし、8ビットの色などをインターリーブしたいと思うと、すぐに面倒になります。

```js
const numVertices = ...;
const npmFloatsPerVertex = 3 + 3 + 2 + 1; // pos + nrm + uv + color()
const f32Data = new Float32Array(numFloatsPerVertex * numVertices);
const u8Data = new Uint8Array(f32Data.buffer);
const colorOffset = (3 + 3 + 2) * 4;

for (let i = 0; i < numVertices; ++i) {
   const floatOffset = numFloatsPerVertex * i;
   f32Data.set(
      [
        x, y, z,
        nx, ny, nz,
        u, v,
      ],
      floatOffset);
   const u8Offset =numFloatsPerVertex * i * 4 + colorOffset;
   u8Data.set(
      [ r, g, b, a ],
      u8Offset;
   );
}
```

そして、まだ終わっていません。そのすべてのデータをバッファに入れたと仮定すると、まだパイプラインを設定する必要があります。

```js
  const pipeline = device.createRenderPipeline({
    vertex: {
      module: shaderModule,
*      buffers: [
*        // position
*        {
*          arrayStride: (3 + 3 + 2 + 1) * 4,
*          attributes: [
*            {shaderLocation: 0, offset: 0,  format: 'float32x3'},
*            {shaderLocation: 1, offset: 12, format: 'float32x3'},
*            {shaderLocation: 2, offset: 24, format: 'float32x2'},
*            {shaderLocation: 3, offset: 32, format: 'unorm8x4'},
*          ],
*        },
*      ],
    ...
```

したがって、いくつかのヘルパーを作成すると、この面倒さを取り除くことができます。

これを渡す関数を作成できます。

```js
const positions = [];
const normals = [];
const texcoords = [];

const data = {
  positions,
  normals,
  texcoords,
};
```

そして、それは私たちのためにすべてを作成します。データをインターリーブし、バッファを作成し、パイプラインの`buffers`部分を返します。

```js
const {
  bufferLayouts,
  buffers,
  numElements
} = createBuffersAndAttributesFromArrays(device, data);
```

これで、バッファはすでに作成されており、デフォルトでは1つしかなく、データはインターリーブされています。そのバッファは`buffer[0]`です。また、パイプラインのバッファと呼ばれる部分である`bufferLayout`も返しました。

```js
  const pipeline = device.createRenderPipeline({
    vertex: {
      module: shaderModule,
*      buffers: bufferLayout
    },
    ...
```

そして、`buffers`が配列であることを考えると、必要に応じて、次のようなバッファコマンドを記述できます。

```js
    const pass = encoder.beginRenderPass(renderPassDescriptor);
    buffers.forEach((buffer, i) => pass.setVertexBuffer(i, buffer));
    ...
```

次に、バッファがさらにあるかどうかにかかわらず、コードを変更する必要はありません。

TBD: 例が必要です。既存の例には、単純でありながら興味深い頂点データが十分にありません。[webgpu-cube](../webgpu-cube.html)を除いて、これはWebGLからのWebGPUに関する記事の一部であり、不適切に思えます。

ただし、かなり良い比較です。

<div class="webgpu_center compare">
  <div>
    <div>Raw WebGPU</div>
<pre class="prettyprint lang-javascript"><code>{{#escapehtml}}
  function createBuffer(device, data, usage) {
    const buffer = device.createBuffer({
      size: data.byteLength,
      usage,
      mappedAtCreation: true,
    });
    const dst = new data.constructor(buffer.getMappedRange());
    dst.set(data);
    buffer.unmap();
    return buffer;
  }

  const positions = new Float32Array([1, 1, -1, 1, 1, 1, 1, -1, 1, 1, -1, -1, -1, 1, 1, -1, 1, -1, -1, -1, -1, -1, -1, 1, -1, 1, 1, 1, 1, 1, 1, 1, -1, -1, 1, -1, -1, -1, -1, 1, -1, -1, 1, -1, 1, -1, -1, 1, 1, 1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1, -1, 1, -1, 1, 1, -1, 1, -1, -1, -1, -1, -1]);
  const normals   = new Float32Array([1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, -1]);
  const texcoords = new Float32Array([1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1]);
  const indices   = new Uint16Array([0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23]);

  const positionBuffer = createBuffer(device, positions, GPUBufferUsage.VERTEX);
  const normalBuffer = createBuffer(device, normals, GPUBufferUsage.VERTEX);
  const texcoordBuffer = createBuffer(device, texcoords, GPUBufferUsage.VERTEX);
  const indicesBuffer = createBuffer(device, indices, GPUBufferUsage.INDEX);

  const pipeline = device.createRenderPipeline({
    label: 'fake lighting',
    layout: 'auto',
    vertex: {
      module: shaderModule,
      buffers: [
        // position
        {
          arrayStride: 3 * 4, // 3 floats, 4 bytes each
          attributes: [
            {shaderLocation: 0, offset: 0, format: 'float32x3'},
          ],
        },
        // normals
        {
          arrayStride: 3 * 4, // 3 floats, 4 bytes each
          attributes: [
            {shaderLocation: 1, offset: 0, format: 'float32x3'},
          ],
        },
        // texcoords
        {
          arrayStride: 2 * 4, // 2 floats, 4 bytes each
          attributes: [
            {shaderLocation: 2, offset: 0, format: 'float32x2',},
          ],
        },
      ],
    },
    fragment: {
      module: shaderModule,
      targets: [
        {format: presentationFormat},
      ],
    },
    primitive: {
      topology: 'triangle-list',
      cullMode: 'back',
    },
    depthStencil: {
      depthWriteEnabled: true,
      depthCompare: 'less',
      format: 'depth24plus',
    },
    ...(canvasInfo.sampleCount > 1 && {
        multisample: {
          count: canvasInfo.sampleCount,
        },
    }),
  });

  ...

    const commandEncoder = device.createCommandEncoder();
    const passEncoder = commandEncoder.beginRenderPass(renderPassDescriptor);
    passEncoder.setPipeline(pipeline);
    passEncoder.setBindGroup(0, bindGroup);
    passEncoder.setVertexBuffer(0, positionBuffer);
    passEncoder.setVertexBuffer(1, normalBuffer);
    passEncoder.setVertexBuffer(2, texcoordBuffer);
    passEncoder.setIndexBuffer(indicesBuffer, 'uint16');
    passEncoder.drawIndexed(indices.length);
    passEncoder.end();
    device.queue.submit([commandEncoder.finish()]);
{{/escapehtml}}</code></pre>
  </div>
  <div>
    <div>WebGPU Utils</div>
<pre class="prettyprint lang-javascript"><code>{{#escapehtml}}
  const {
    buffers: [vertexBuffer],
    bufferLayouts,
    indexBuffer,
    indexFormat,
    numElements,
  } = createBuffersAndAttributesFromArrays(
    device, {
      positions: [1, 1, -1, 1, 1, 1, 1, -1, 1, 1, -1, -1, -1, 1, 1, -1, 1, -1, -1, -1, -1, -1, -1, 1, -1, 1, 1, 1, 1, 1, 1, 1, -1, -1, 1, -1, -1, -1, -1, 1, -1, -1, 1, -1, 1, -1, -1, 1, 1, 1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1, -1, 1, -1, 1, 1, -1, 1, -1, -1, -1, -1, -1],
      normals: [1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0, 0, -1],
      texcoords: [1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1],
      indices: [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23],
    });

  const pipeline = device.createRenderPipeline({
    label: 'fake lighting',
    layout: 'auto',
    vertex: {
      module: shaderModule,
      buffers: bufferLayouts,
    },
    fragment: {
      module: shaderModule,
      targets: [
        {format: presentationFormat},
      ],
    },
    primitive: {
      topology: 'triangle-list',
      cullMode: 'back',
    },
    depthStencil: {
      depthWriteEnabled: true,
      depthCompare: 'less',
      format: 'depth24plus',
    },
    ...(canvasInfo.sampleCount > 1 && {
        multisample: {
          count: canvasInfo.sampleCount,
        },
    }),
  });

...

    const commandEncoder = device.createCommandEncoder();
    const passEncoder = commandEncoder.beginRenderPass(renderPassDescriptor);
    passEncoder.setPipeline(pipeline);
    passEncoder.setBindGroup(0, bindGroup);
    passEncoder.setVertexBuffer(0, vertexBuffer);
    passEncoder.setIndexBuffer(indexBuffer, indexFormat);
    passEncoder.drawIndexed(numElements);
    passEncoder.end();
    device.queue.submit([commandEncoder.finish()]);
{{/escapehtml}}</code></pre>
  </div>
</div>


[頂点バッファに関する記事](webgpu-vertex-buffers.html#a-normalized-attributes)の8ビットの色を使用する例のような、より複雑な例はどうでしょうか。3つのバッファがありました。1つは位置と頂点ごとの色用です。1つは円ごとの色と円ごとのオフセット用で、最後の1つはスケール用です。

`createBuffersAndAttributesFromArrays`を使用するように変更します。

まず、円データを作成するコードを変更します。

```js
function createCircleVertices({
  radius = 1,
  numSubdivisions = 24,
  innerRadius = 0,
  startAngle = 0,
  endAngle = Math.PI * 2,
} = {}) {
-  // 1つのサブディビジョンあたり2つの三角形、1つの三角形あたり3つの頂点
-  const numVertices = numSubdivisions * 3 * 2;
-  // 位置（xy）に2つの32ビット値、色（rgb_）に1つの32ビット値
-  // 32ビットの色値は、4つの8ビット値として書き込み/読み取りされます
-  const vertexData = new Float32Array(numVertices * (2 + 1));
-  const colorData = new Uint8Array(vertexData.buffer);

+  const positions = [];
+  const colors = [];

-  let offset = 0;
-  let colorOffset = 8;
  const addVertex = (x, y, r, g, b) => {
-    vertexData[offset++] = x;
-    vertexData[offset++] = y;
-    offset += 1;  // 色をスキップします
-    colorData[colorOffset++] = r * 255;
-    colorData[colorOffset++] = g * 255;
-    colorData[colorOffset++] = b * 255;
-    colorOffset += 9;  // 余分なバイトと位置をスキップします
+    positions.push(x, y);
+    colors.push(r, g, b, 1);
  };

  const innerColor = [1, 1, 1];
  const outerColor = [0.1, 0.1, 0.1];

  // 1つのサブディビジョンあたり2つの頂点
  //
  // 0--1 4
  // | / /|
  // |/ / |
  // 2 3--5
  for (let i = 0; i < numSubdivisions; ++i) {
    const angle1 = startAngle + (i + 0) * (endAngle - startAngle) / numSubdivisions;
    const angle2 = startAngle + (i + 1) * (endAngle - startAngle) / numSubdivisions;

    const c1 = Math.cos(angle1);
    const s1 = Math.sin(angle1);
    const c2 = Math.cos(angle2);
    const s2 = Math.sin(angle2);

    // 最初の三角形
    addVertex(c1 * radius, s1 * radius, ...outerColor);
    addVertex(c2 * radius, s2 * radius, ...outerColor);
    addVertex(c1 * innerRadius, s1 * innerRadius, ...innerColor);

    // 2番目の三角形
    addVertex(c1 * innerRadius, s1 * innerRadius, ...innerColor);
    addVertex(c2 * radius, s2 * radius, ...outerColor);
    addVertex(c2 * innerRadius, s2 * innerRadius, ...innerColor);
  }

  return {
-    vertexData,
-    numVertices,
+    positions: { data: positions, numComponents: 2 },
+    colors,
  };
}
```

それで、それはより単純になりました。

頂点バッファを設定するコードは、次のようになります。

```
  const kNumObjects = 100;
  const objectInfos = [];

-  // 2つの頂点バッファを作成します
-  const staticUnitSize =
-    4 +     // colorは4バイトです
-    2 * 4;  // offsetは2つの32ビット浮動小数点数（各4バイト）です
-  const changingUnitSize =
-    2 * 4;  // scaleは2つの32ビット浮動小数点数（各4バイト）です
-  const staticVertexBufferSize = staticUnitSize * kNumObjects;
-  const changingVertexBufferSize = changingUnitSize * kNumObjects;
-
-  const staticVertexBuffer = device.createBuffer({
-    label: 'static vertex for objects',
-    size: staticVertexBufferSize,
-    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
-  });
-
-  const changingVertexBuffer = device.createBuffer({
-    label: 'changing storage for objects',
-    size: changingVertexBufferSize,
-    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
-  });
-
-  // float32インデックスでのさまざまなユニフォーム値へのオフセット
-  const kColorOffset = 0;
-  const kOffsetOffset = 1;

  const kScaleOffset = 0;

-  {
-    const staticVertexValuesU8 = new Uint8Array(staticVertexBufferSize);
-    const staticVertexValuesF32 = new Float32Array(staticVertexValuesU8.buffer);
+  const staticColors = [];
+  const staticOffsets = [];

    for (let i = 0; i < kNumObjects; ++i) {
-      const staticOffsetU8 = i * staticUnitSize;
-      const staticOffsetF32 = staticOffsetU8 / 4;
-
-      // これらは一度だけ設定されるので、今すぐ設定します
-      staticVertexValuesU8.set(        // 色を設定します
-          [rand() * 255, rand() * 255, rand() * 255, 255],
-          staticOffsetU8 + kColorOffset);
-
-      staticVertexValuesF32.set(      // オフセットを設定します
-          [rand(-0.9, 0.9), rand(-0.9, 0.9)],
-          staticOffsetF32 + kOffsetOffset);
+      staticColors.push(rand() * 255, rand() * 255, rand() * 255, 255);
+      staticOffsets.push(rand(-0.9, 0.9), rand(-0.9, 0.9));

      objectInfos.push({
        scale: rand(0.2, 0.5),
      });
    }
-    device.queue.writeBuffer(staticVertexBuffer, 0, staticVertexValuesF32);
-  }

  const {
    buffers: [staticVertexBuffer],
    bufferLayouts: [staticVertexBufferLayout],
  } = createBuffersAndAttributesFromArrays(device, {
    staticOffsets: { data: staticOffsets, numComponents: 2 },
    staticColors: new Uint8Array(staticColors),
  }, {stepMode: 'instance', shaderLocation: 2});

  const {
    buffers: [changingVertexBuffer],
    bufferLayouts: [changingVertexBufferLayout],
  } = createBuffersAndAttributesFromArrays(device, {
    scale: { data: kNumObjects * 2, numComponents: 2 },
  }, { stepMode: 'instance', shaderLocation: 4, usage: GPUBufferUsage.COPY_DST });

+  const vertexValues = new Float32Array(changingVertexBuffer.size / 4);
+  const changingUnitSize = 8;

-  // changingStorageBufferを更新するために使用できる型付き配列
-  const vertexValues = new Float32Array(changingVertexBufferSize / 4);
-
-  const { vertexData, numVertices } = createCircleVertices({
-    radius: 0.5,
-    innerRadius: 0.25,
-  });
-  const vertexBuffer = device.createBuffer({
-    label: 'vertex buffer vertices',
-    size: vertexData.byteLength,
-    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
-  });
-  device.queue.writeBuffer(vertexBuffer, 0, vertexData);

+  const vertexArrays = createCircleVertices({
+    radius: 0.5,
+    innerRadius: 0.25,
+  });
+  const {
+    buffers: [vertexBuffer],
+    numElements,
+    bufferLayouts: [vertexBufferLayout],
+  } = createBuffersAndAttributesFromArrays(device, vertexArrays);
```

それははるかに小さくなりました。

パイプラインを設定するコードは次のようになります。

```js
  const pipeline = device.createRenderPipeline({
    label: 'per vertex color',
    layout: 'auto',
    vertex: {
      module,
      buffers: [
-        {
-          arrayStride: 2 * 4 + 4, // 2 floats, 4 bytes each + 4 bytes
-          attributes: [
-            {shaderLocation: 0, offset: 0, format: 'float32x2'},  // position
-            {shaderLocation: 4, offset: 8, format: 'unorm8x4'},   // perVertexColor
-          ],
-        },
-        {
-          arrayStride: 4 + 2 * 4, // 4 bytes + 2 floats, 4 bytes each
-          stepMode: 'instance',
-          attributes: [
-            {shaderLocation: 1, offset: 0, format: 'unorm8x4'},   // color
-            {shaderLocation: 2, offset: 4, format: 'float32x2'},  // offset
-          ],
-        },
-        {
-          arrayStride: 2 * 4, // 2 floats, 4 bytes each
-          stepMode: 'instance',
-          attributes: [
-            {shaderLocation: 3, offset: 0, format: 'float32x2'},   // scale
-          ],
-        },
+        vertexBufferLayout,
+        staticVertexBufferLayout,
+        changingVertexBufferLayout,
      ],
    },
    fragment: {
      module,
      targets: [{ format: presentationFormat }],
    },
  });
```

それで、それはより単純です。

それは勝利ですか？あなたが決める必要があります。

ただし、今後は、一部の例では、記事が本当に何についてであるかに集中するために、これらの関数を使用し始めます。これらの詳細で雑草に迷うのではなく。この記事が、これらの関数が何をするのかをより明確にするのに役立つことを願っています。それらは、すでに説明されていること以外は何も行いません。したがって、次のようなものを見たとき、

```js
const sphereData = createBuffersAndAttributesFromArrays(
   device,
   createSphereVertices(radius),
);
```

このサイトには、`createBuffersAndAttributesFromArrays`が何を意味するのかを説明する30〜40の記事があり、これらのユーティリティについて怖いことや理解しにくいことは何もないことを願っています。概念を説明し、それに名前を付け、名前で参照するだけというのは、学習の標準です。これにより、より高いレベルの概念をより簡単に構築できます。