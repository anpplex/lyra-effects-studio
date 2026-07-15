# Registry 发布运维手册

本文供维护者发布 Lyra 使用的静态签名 Theme Registry。Ed25519 私钥必须保存在仓库之外；生产 APK 绝不能使用本地测试 key。

## 1. 离线生成生产签名材料

在可信维护机上生成一个随机的 32 字节 Ed25519 seed：

```sh
openssl rand -base64 32 | tr -d '\n' > lyra-registry-private-key.base64
chmod 600 lyra-registry-private-key.base64
```

发布脚本通过 `LYRA_REGISTRY_PRIVATE_KEY_BASE64` 读取这个值。本地构建会将对应公钥写入 `public-key.txt`；Android 构建只需要审核后的公钥，私钥不能上传。

## 2. 配置 GitHub

每个仓库只需把 Pages source 设置为 **GitHub Actions** 一次，然后将私钥 seed 写入受保护的 Actions Secret，并保持官方 key ID（也可以使用 workflow 默认值）：

```sh
gh api --method POST repos/anpplex/lyra-effects-studio/pages -f build_type=workflow
gh secret set LYRA_REGISTRY_PRIVATE_KEY_BASE64 \
  --repo anpplex/lyra-effects-studio \
  < lyra-registry-private-key.base64
gh variable set LYRA_REGISTRY_KEY_ID \
  --repo anpplex/lyra-effects-studio \
  --body lyra-official-v1
```

`Publish Theme Registry` workflow 在 `v*` tag 或手动 dispatch 时运行。它会先检查 Secret，再执行源码、许可证和可复现性门禁，签名目录与 Pack checksum，校验完整站点并部署以下文件：`registry-v1.json`、`registry-v1.sig`、`public-key.txt` 及版本化 Pack。

发布地址：

```text
https://anpplex.github.io/lyra-effects-studio/
```

## 3. 把公钥注入 Lyra

发布成功后，将线上 `public-key.txt` 与线下审核公钥比对。使用精确的 base64 值构建 Lyra；默认构建会保持在线 Registry 控件禁用：

```sh
./gradlew \
  -PlyraRegistryPublicKeyBase64="$(tr -d '\n' < public-key.txt)" \
  :app:assembleRelease
```

Android 客户端固定 origin，验证目录和 Pack 签名，并且只有用户按下 **刷新目录** 后才启动网络请求。不要通过远端配置修改 origin 或公钥。

## 4. 轮换 key

轮换是协同发布流程：

1. 离线生成新 seed，并按正常评审流程记录新公钥。
2. 更新 GitHub protected Secret 和 `LYRA_REGISTRY_KEY_ID` variable。
3. 发布新的 Registry 站点，核对 `public-key.txt` 与全部签名。
4. 先发布包含新公钥的 Android 版本，再下线旧分发。

私钥、生产 `public-key.txt` 副本或生成后的 Registry 压缩包都不能提交进 Git。
