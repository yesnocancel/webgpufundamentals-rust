Title: WebGPU 工作原理
Description: WebGPU 的工作原理
TOC: 工作原理

让我们通过在 JavaScript 中实现一个与 GPU 使用顶点着色器和片段着色器所做的类似的事情来解释 WebGPU。希望这能让你对 GPU 实际发生的事情有一个直观的感受。

如果你熟悉
[Array.map](https://developer.mozilla.org/zh_CN/docs/Web/JavaScript/Reference/Global_Objects/Array/map)，
如果眯着眼睛仔细看，你可以得到一些关于这两种不同类型着色器函数工作方式的认识。
使用 `Array.map`，你提供一个用于转换值的函数。

示例：

```js
const shader = v => v * 2;  // 将输入值翻倍
const input = [1, 2, 3, 4];
const output = input.map(shader);   // 结果 [2, 4, 6, 8]
```

上面的"着色器"（shader）对于 array.map 来说只是一个接收一个数字并返回其双倍值的函数。
这可能是 JavaScript 中最接近"着色器"含义的类比了。
它是一个返回或生成值的函数。你不会直接调用它。
相反，你只需指定它，然后系统会为你调用它。

对于 GPU 顶点着色器，你不是对一个输入数组进行映射操作。
相反，你只需指定你想要这个函数被调用的次数。

```js
function draw(count, vertexShaderFn) {
  const internalBuffer = [];
  for (let i = 0; i < count; ++i) {
    internalBuffer[i] = vertexShaderFn(i);
  }
  console.log(JSON.stringify(internalBuffer));
}
```

一个后果是，与 `Array.map` 不同，我们不再需要一个源数组来做某些事情。

```js
const shader = v => v * 2;
const count = 4;
draw(count, shader);
// 输出 [0, 2, 4, 6]
```

让 GPU 工作变得复杂的原因是，这些函数运行在你电脑上另一个独立的系统——GPU 上。
这意味着你创建和引用的所有数据必须以某种方式发送到 GPU，
然后你还需要告诉着色器你把数据放在哪里以及如何访问它。

顶点着色器和片段着色器可以通过 6 种方式获取数据：
Uniforms（统一变量）、Attributes（属性）、Buffers（缓冲区）、Textures（纹理）、
Inter-Stage Variables（阶段间变量）、Constants（常量）。

1. Uniforms（统一变量）

   Uniforms 是对于着色器的每次迭代都相同的值。可以把它们想象成常量全局变量。
   你可以在着色器运行之前设置它们，但是当着色器被使用时，它们保持不变，
   或者换句话说，它们保持*统一*（uniform）。

   让我们修改 `draw`，将 uniforms 传递给着色器。
   为此，我们需要创建一个名为 `bindings` 的数组，并用它在着色器之间传递数据。

   ```js
   *function draw(count, vertexShaderFn, bindings) {
     const internalBuffer = [];
     for (let i = 0; i < count; ++i) {
   *    internalBuffer[i] = vertexShaderFn(i, bindings);
     }
     console.log(JSON.stringify(internalBuffer));
   }
   ```

   然后让我们修改着色器来使用 uniforms

   ```js
   const vertexShader = (v, bindings) => {
     const uniforms = bindings[0];
     return v * uniforms.multiplier;
   };
   const count = 4;
   const uniforms1 = {multiplier: 3};
   const uniforms2 = {multiplier: 5};
   const bindings1 = [uniforms1];
   const bindings2 = [uniforms2];
   draw(count, vertexShader, bindings1);
   // 输出 [0, 3, 6, 9]
   draw(count, vertexShader, bindings2);
   // 输出 [0, 5, 10, 15]
   ```

   因此，uniforms 的概念应该相当直接。
   通过 `bindings` 的间接寻址是因为这与 WebGPU 中的做法"类似"。
   正如上面提到的，我们通过位置/索引来访问东西，
   在这里它们在 `bindings[0]` 中被找到。

2. Attributes（属性，仅限顶点着色器）

   Attributes 为着色器的每次迭代提供数据。
   在上面的 `Array.map` 中，值 `v` 是从 `input` 中提取的，
   并自动提供给函数。这与着色器中的 attribute 非常相似。

   不同之处在于，我们不是对输入进行映射，
   相反，因为我们只是计数，所以我们需要告诉 WebGPU
   这些输入是什么以及如何从中获取数据。

   想象我们像这样更新 `draw`。

   ```js
   *function draw(count, vertexShaderFn, bindings, attribsSpec) {
     const internalBuffer = [];
     for (let i = 0; i < count; ++i) {
   *    const attribs = getAttribs(attribsSpec, i);
   *    internalBuffer[i] = vertexShaderFn(i, bindings, attribs);
     }
     console.log(JSON.stringify(internalBuffer));
   }

   +function getAttribs(attribs, ndx) {
   +  return attribs.map(({source, offset, stride}) => source[ndx * stride + offset]);
   +}
   ```

   然后我们可以这样调用它。

   ```js
   const buffer1 = [0, 1, 2, 3, 4, 5, 6, 7];
   const buffer2 = [11, 22, 33, 44];
   const attribsSpec = [
     { source: buffer1, offset: 0, stride: 2, },
     { source: buffer1, offset: 1, stride: 2, },
     { source: buffer2, offset: 0, stride: 1, },
   ];
   const vertexShader = (v, bindings, attribs) => (attribs[0] + attribs[1]) * attribs[2];
   const bindings = [];
   const count = 4;
   draw(count, vertexShader, bindings, attribsSpec);
   // 输出 [11, 110, 297, 572]
   ```

   如你所见，上面 `getAttribs` 使用 `offset` 和 `stride` 来
   计算对应 `source` 缓冲区中的索引，并提取值。
   提取的值然后被发送到着色器。每次迭代中
   `attribs` 都会不同。

   ```
    迭代次数 |  attribs
    ----------+-------------
        0     | [0, 1, 11]
        1     | [2, 3, 22]
        2     | [4, 5, 33]
        3     | [6, 7, 44]
   ```

3. Raw Buffers（原始缓冲区）

   缓冲区实际上是数组。同样，为了我们的类比，让我们创建一个使用缓冲区的 `draw` 版本。
   我们将通过 `bindings` 传递这些缓冲区，就像我们对 uniforms 做的那样。

   ```js
   const buffer1 = [0, 1, 2, 3, 4, 5, 6, 7];
   const buffer2 = [11, 22, 33, 44];
   const attribsSpec = [];
   const bindings = [
     buffer1,
     buffer2,
   ];
   const vertexShader = (ndx, bindings, attribs) => 
       (bindings[0][ndx * 2] + bindings[0][ndx * 2 + 1]) * bindings[1][ndx];
   const count = 4;
   draw(count, vertexShader, bindings, attribsSpec);
   // 输出 [11, 110, 297, 572]
   ```

   这里我们得到了与使用 attributes 时相同的结果，
   不同的是，这次不是系统为我们从缓冲区中提取值，
   而是我们自己计算索引来访问绑定的缓冲区。
   这比 attributes 更灵活，因为本质上我们可以随机访问数组。
   但正因为如此，它也可能更慢。
   attributes 工作方式的一个好处是，GPU 知道这些值将按顺序访问，
   这可以被用来进行优化。例如，按顺序访问通常对缓存友好。
   当我们计算自己的索引时，GPU 不知道我们将要访问缓冲区的哪一部分，
   直到我们实际尝试访问它。

4. Textures（纹理）

   纹理是一维、二维或三维的数据数组。当然，我们可以使用缓冲区来实现自己的二维或三维数组。
   纹理的特殊之处在于它们可以被采样。
   采样意味着我们可以要求 GPU 计算我们提供的值之间的值。
   我们将在[关于纹理的文章](webgpu-textures.html)中详细解释这意味着什么。
   现在，让我们再用一个 JavaScript 类比来理解它。

   首先，我们将创建一个函数 `textureSample`，它可以*采样*数组中任意位置的值。

   ```js
   function textureSample(texture, ndx) {
     const startNdx = ndx | 0;  // 向下取整
     const fraction = ndx % 1;  // 获取索引之间的分数部分
     const start = texture[startNdx];
     const end = texture[startNdx + 1];
     return start + (end - start) * fraction;  // 计算中间值
   }
   ```

   在 GPU 上已经存在一个类似的函数。

   现在让我们在着色器中使用它。

   ```js
   const texture = [10, 20, 30, 40, 50, 60, 70, 80];
   const attribsSpec = [];
   const bindings = [
     texture,
   ];
   const vertexShader = (ndx, bindings, attribs) =>
       textureSample(bindings[0], ndx * 1.75);
   const count = 4;
   draw(count, vertexShader, bindings, attribsSpec);
   // 输出 [10, 27.5, 45, 62.5]
   ```

   当 `ndx` 是 `3` 时，我们将 `3 * 1.75` 即 `5.25` 传入 `textureSample`。
   这将计算 `startNdx` 为 `5`。所以我们会提取索引 `5` 和 `6`，
   即 `60` 和 `70`。`fraction` 变成 `0.25`，
   所以我们会得到 `60 + (70 - 60) * 0.25`，即 `62.5`。

   看上面的代码，我们可以自己在着色器函数中写 `textureSample`。
   我们可以手动提取这两个值并在它们之间插值。
   GPU 有这个特殊功能的原因是它可以做得快得多，
   而且，根据设置的不同，它可能读取多达十六个 4 浮点数值
   来为我们生成一个 4 浮点数值。手动做这件事会花费很多工作。

5. Inter-Stage Variables（阶段间变量，仅限片段着色器）

   阶段间变量是顶点着色器传递给片段着色器的输出值。
   正如上面提到的，顶点着色器输出用于绘制/光栅化点、线和三角形的坐标。

   假设我们正在画一条线。假设我们的顶点着色器运行了两次，
   第一次输出 `5,0`，第二次输出 `25,4`。
   给定这两个点，GPU 将在 `5,0` 到 `25,4` 之间画一条线（不含终点）。
   为此，它将调用我们的片段着色器 20 次，每个线上的像素一次。
   每次调用片段着色器时，由我们来决定返回什么颜色。

   假设我们有一对函数来帮助我们在两点之间画线。
   第一个函数计算我们需要画多少个像素和一些帮助绘制它们的值。
   第二个函数接收这些信息加上一个像素编号，并给我们一个像素位置。示例：

   ```js
   const line = calcLine([10, 10], [13, 13]);
   for (let i = 0; i < line.numPixels; ++i) {
     const p = calcLinePoint(line, i);
     console.log(p);
   }
   // 打印
   // 10,10
   // 11,11
   // 12,12
   ```

   注意：`calcLine` 和 `calcLinePoint` 的具体实现并不重要，
   重要的是它们确实有作用，并让上面的循环能够提供画线所需的像素位置。
   **不过，如果你好奇，可以在文章底部的实际代码示例中看到它们的实现。**

   那么，让我们修改顶点着色器，使其每次迭代输出 2 个值。我们可以用很多种方式做到这一点。这里是其中一种。

   ```js
   const buffer1 = [5, 0, 25, 4];
   const attribsSpec = [
     {source: buffer1, offset: 0, stride: 2},
     {source: buffer1, offset: 1, stride: 2},
   ];
   const bindings = [];
   const dest = new Array(2);
   const vertexShader = (ndx, bindings, attribs) => [attribs[0], attribs[1]];
   const count = 2;
   draw(count, vertexShader, bindings, attribsSpec);
   // 输出 [[5, 0], [25, 4]]
   ```

   现在让我们写一些代码，遍历每 2 个点，
   调用 `rasterizeLines` 来光栅化一条线。

   ```js
   function rasterizeLines(dest, destWidth, inputs, fragShaderFn, bindings) {
     for (let ndx = 0; ndx < inputs.length - 1; ndx += 2) {
       const p0 = inputs[ndx    ];
       const p1 = inputs[ndx + 1];
       const line = calcLine(p0, p1);
       for (let i = 0; i < line.numPixels; ++i) {
         const p = calcLinePoint(line, i);
         const offset = p[1] * destWidth + p[0];  // y * width + x
         dest[offset] = fragShaderFn(bindings);
       }
     }
   }
   ```

   我们可以像这样更新 `draw` 来使用那段代码

   ```js
   -function draw(count, vertexShaderFn, bindings, attribsSpec) {
   +function draw(dest, destWidth,
   +              count, vertexShaderFn, fragmentShaderFn,
   +              bindings, attribsSpec,
   +) {
     const internalBuffer = [];
     for (let i = 0; i < count; ++i) {
       const attribs = getAttribs(attribsSpec, i);
       internalBuffer[i] = vertexShaderFn(i, bindings, attribs);
     }
   -  console.log(JSON.stringify(internalBuffer));
   +  rasterizeLines(dest, destWidth, internalBuffer,
   +                 fragmentShaderFn, bindings);
   }
   ```

   现在我们实际上在使用 `internalBuffer` 😃！

   让我们更新调用 `draw` 的代码。

   ```js
   const buffer1 = [5, 0, 25, 4];
   const attribsSpec = [
     {source: buffer1, offset: 0, stride: 2},
     {source: buffer1, offset: 1, stride: 2},
   ];
   const bindings = [];
   const vertexShader = (ndx, bindings, attribs) => [attribs[0], attribs[1]];
   const count = 2;
   -draw(count, vertexShader, bindings, attribsSpec);

   +const width = 30;
   +const height = 5;
   +const pixels = new Array(width * height).fill(0);
   +const fragShader = (bindings) => 6;

   *draw(
   *   pixels, width,
   *   count, vertexShader, fragShader,
   *   bindings, attribsSpec);
   ```

   如果我们将 `pixels` 打印成一个矩形，其中 `0` 变成 `.`，我们会得到这样的结果

   ```
   .....666......................
   ........66666.................
   .............66666............
   ..................66666.......
   .......................66.....
   ```

   不幸的是，我们的片段着色器没有收到每次迭代都会变化的输入，
   所以没有办法为每个像素输出不同的东西。
   这就是阶段间变量发挥作用的地方。
   让我们修改第一个着色器，让它输出一个额外的值。

   ```js
   const buffer1 = [5, 0, 25, 4];
   +const buffer2 = [9, 3];
   const attribsSpec = [
     {source: buffer1, offset: 0, stride: 2},
     {source: buffer1, offset: 1, stride: 2},
   +  {source: buffer2, offset: 0, stride: 1},
   ];
   const bindings = [];
   const dest = new Array(2);
   const vertexShader = (ndx, bindings, attribs) => 
   -    [attribs[0], attribs[1]];
   +    [[attribs[0], attribs[1]], [attribs[2]]];

   ...
   ```

   如果我们不改变其他东西，在 `draw` 内部的循环之后，
   `internalBuffer` 将包含这些值

   ```js
    [ 
      [[ 5, 0], [9]],
      [[25, 4], [3]],
    ]
   ```

   我们可以轻松计算一个从 0.0 到 1.0 的值，表示我们在线上的位置。
   我们可以用这个值来对我们刚刚添加的额外值进行插值。

   ```js
   function rasterizeLines(dest, destWidth, inputs, fragShaderFn, bindings) {
     for(let ndx = 0; ndx < inputs.length - 1; ndx += 2) {
   -    const p0 = inputs[ndx    ];
   -    const p1 = inputs[ndx + 1];
   +    const p0 = inputs[ndx    ][0];
   +    const p1 = inputs[ndx + 1][0];
   +    const v0 = inputs[ndx    ].slice(1);  // 除了第一个值之外的所有值
   +    const v1 = inputs[ndx + 1].slice(1);
       const line = calcLine(p0, p1);
       for (let i = 0; i < line.numPixels; ++i) {
         const p = calcLinePoint(line, i);
   +      const t = i / line.numPixels;
   +      const interStageVariables = interpolateArrays(v0, v1, t);
         const offset = p[1] * destWidth + p[0];  // y * width + x
   -      dest[offset] = fragShaderFn(bindings);
   +      dest[offset] = fragShaderFn(bindings, interStageVariables);
       }
     }
   }

   +// interpolateArrays([[1,2]], [[3,4]], 0.25) => [[1.5, 2.5]]
   +function interpolateArrays(v0, v1, t) {
   +  return v0.map((array0, ndx) => {
   +    const array1 = v1[ndx];
   +    return interpolateValues(array0, array1, t);
   +  });
   +}

   +// interpolateValues([1,2], [3,4], 0.25) => [1.5, 2.5]
   +function interpolateValues(array0, array1, t) {
   +  return array0.map((a, ndx) => {
   +    const b = array1[ndx];
   +    return a + (b - a) * t;
   +  });
   +}
   ```

   现在我们可以在片段着色器中使用这些阶段间变量了

   ```js
   -const fragShader = (bindings) => 6;
   +const fragShader = (bindings, interStageVariables) => 
   +    interStageVariables[0] | 0; // 转换为整数
   ```

   如果我们现在运行它，我们会看到这样的结果

   ```
   .....988......................
   ........87776.................
   .............66655............
   ..................54443........
   .......................33.....
   ```

   顶点着色器的第一次迭代输出了 `[[5,0], [9]]`，
   第二次迭代输出了 `[[25,4], [3]]`。
   你可以看到，当片段着色器被调用时，
   这两个值中的第二个值在两个值之间进行了插值。

   我们可以再创建一个函数 `mapTriangle`，给定 3 个点，
   光栅化一个三角形，为三角形内的每个点调用片段着色器函数。
   它将在 3 个点之间而不是 2 个点之间插值阶段间变量。

以下是上述所有示例的在线运行版本，
如果你觉得玩弄它们有助于理解它们，可能会对你有用。

{{{example url="../webgpu-javascript-analogies.html"}}}

上面的 JavaScript 中的内容是一个类比。
阶段间变量实际如何插值、线如何绘制、
缓冲区如何访问、纹理如何采样、uniforms 如何设置、
attributes 如何指定等细节，在 WebGPU 中是不同的，
但概念非常相似，所以希望你通过这个 JavaScript 类比
能够帮助你建立一个关于正在发生的事情的心理模型。

为什么是这样的？好吧，如果你看 `draw` 和 `rasterizeLines`，
你可能会注意到每次迭代完全独立于其他迭代。
换句话说，你可以以任何顺序处理每个迭代。
而不是 0、1、2、3、4，你可以处理它们 3、1、4、0、2，
你会得到完全相同的结果。
正因为它们是独立的，每个迭代都可以由不同的处理器并行运行。
2021 年的顶级 GPU 有 10000 或更多个处理器。
这意味着最多可以有 10000 个东西并行运行。
这就是使用 GPU 的强大之处所在。
通过遵循这些模式，系统可以大规模地并行化工作。

最大的限制是：

1. 着色器函数只能引用它的输入
   （attributes、buffers、textures、uniforms、阶段间变量）。

2. 着色器不能分配内存。

3. 着色器在引用它写入的东西（它正在生成值的目标）时必须非常小心。

   仔细想想这很有道理。想象上面的 `fragShader`
   试图直接引用 `dest`。
   这意味着当试图并行化事情时，不可能协调。
   哪个迭代会先运行？如果第三个迭代引用了 `dest[0]`，
   那么第零个迭代需要先运行，但如果是第零个迭代
   引用了 `dest[3]`，那么第三个迭代需要先运行。

   在 CPU 和多线程或多进程的领域也存在着类似的设计限制，
   但在 GPU 领域，多达 10000 个处理器同时运行，
   需要特殊的协调机制。我们将在其他文章中尝试介绍一些相关技术。
