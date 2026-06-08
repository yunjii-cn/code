package com.yunjii.code;

import android.content.Context;
import android.content.SharedPreferences;
import android.content.pm.PackageInfo;
import android.os.Build;
import android.util.Base64;

import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.Iterator;
import java.util.UUID;

import java.security.KeyStore;

import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;

import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

@CapacitorPlugin(name = "YunJi")
public class YunJiPlugin extends Plugin {

    private static final String PREFS_NAME = "yunji_prefs";
    private static final String SECURE_PREFS_NAME = "yunji_secure_prefs";
    private static final String PROJECTS_KEY = "projects";
    private static final String CONVERSATIONS_KEY_PREFIX = "conversations_";
    private static final String ACTIVE_PROJECT_KEY = "active_project_id";
    private static final String ENCRYPTED_KEY_PREFIX = "enc_";
    private static final String ANDROID_KEYSTORE_ALIAS = "yunji_key";

    private volatile HttpURLConnection currentChatConnection;

    private SharedPreferences getPrefs() {
        return getContext().getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
    }

    private SharedPreferences getSecurePrefs() {
        return getContext().getSharedPreferences(SECURE_PREFS_NAME, Context.MODE_PRIVATE);
    }

    // ==================== AI 对话 ====================

    @PluginMethod
    public void aiChat(PluginCall call) {
        String messagesJson = call.getString("messages", "[]");
        String model = call.getString("model", "");
        String provider = call.getString("provider", "");
        String apiKey = call.getString("api_key", "");
        String baseUrl = call.getString("base_url", "");

        if (apiKey.isEmpty()) {
            call.reject("API Key 不能为空");
            return;
        }

        new Thread(() -> {
            try {
                String apiUrl;
                JSONObject requestBody = new JSONObject();

                switch (provider) {
                    case "anthropic":
                        apiUrl = baseUrl.isEmpty()
                                ? "https://api.anthropic.com/v1/messages"
                                : baseUrl + "/v1/messages";
                        requestBody.put("model", model);
                        requestBody.put("max_tokens", 4096);
                        requestBody.put("stream", true);
                        requestBody.put("messages", new JSONArray(messagesJson));
                        break;

                    case "openrouter":
                        apiUrl = baseUrl.isEmpty()
                                ? "https://openrouter.ai/api/v1/chat/completions"
                                : baseUrl + "/v1/chat/completions";
                        requestBody.put("model", model);
                        requestBody.put("stream", true);
                        requestBody.put("messages", new JSONArray(messagesJson));
                        break;

                    case "zhipu":
                        apiUrl = baseUrl.isEmpty()
                                ? "https://open.bigmodel.cn/api/paas/v4/chat/completions"
                                : baseUrl + "/v1/chat/completions";
                        requestBody.put("model", model);
                        requestBody.put("stream", true);
                        requestBody.put("messages", new JSONArray(messagesJson));
                        break;

                    default:
                        apiUrl = baseUrl.isEmpty()
                                ? "https://openrouter.ai/api/v1/chat/completions"
                                : baseUrl + "/v1/chat/completions";
                        requestBody.put("model", model);
                        requestBody.put("stream", true);
                        requestBody.put("messages", new JSONArray(messagesJson));
                        break;
                }

                URL url = new URL(apiUrl);
                HttpURLConnection conn = (HttpURLConnection) url.openConnection();
                conn.setRequestMethod("POST");
                conn.setRequestProperty("Content-Type", "application/json");
                conn.setDoOutput(true);
                conn.setConnectTimeout(30000);
                conn.setReadTimeout(300000);

                // 设置认证头
                if ("anthropic".equals(provider)) {
                    conn.setRequestProperty("x-api-key", apiKey);
                    conn.setRequestProperty("anthropic-version", "2023-06-01");
                } else {
                    conn.setRequestProperty("Authorization", "Bearer " + apiKey);
                }

                if ("openrouter".equals(provider)) {
                    conn.setRequestProperty("HTTP-Referer", "https://yunjii.code");
                    conn.setRequestProperty("X-Title", "YunJi Code");
                }

                currentChatConnection = conn;

                byte[] bodyBytes = requestBody.toString().getBytes(StandardCharsets.UTF_8);
                try (OutputStream os = conn.getOutputStream()) {
                    os.write(bodyBytes);
                }

                int responseCode = conn.getResponseCode();
                if (responseCode != 200) {
                    String errorMsg = readErrorStream(conn);
                    notifyListeners("ai:chunk", new JSObject().put("error", errorMsg));
                    notifyListeners("ai:end", new JSObject().put("ok", false));
                    call.resolve(new JSObject().put("ok", false));
                    return;
                }

                // 解析 SSE 流
                BufferedReader reader = new BufferedReader(
                        new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8));
                String line;
                StringBuilder buffer = new StringBuilder();

                while ((line = reader.readLine()) != null) {
                    if (Thread.currentThread().isInterrupted()) break;

                    if (line.isEmpty()) {
                        if (buffer.length() > 0) {
                            processSseData(buffer.toString(), provider);
                            buffer.setLength(0);
                        }
                        continue;
                    }

                    if (line.startsWith("data: ")) {
                        String data = line.substring(6).trim();
                        if ("[DONE]".equals(data)) {
                            notifyListeners("ai:end", new JSObject().put("ok", true));
                            break;
                        }
                        buffer.append(data);
                    } else if (!line.startsWith(":") && !line.startsWith("event:")) {
                        buffer.append(line);
                    }
                }

                reader.close();
                notifyListeners("ai:end", new JSObject().put("ok", true));
                call.resolve(new JSObject().put("ok", true));

            } catch (Exception e) {
                if (e instanceof java.net.SocketException || e instanceof java.io.IOException) {
                    notifyListeners("ai:end", new JSObject().put("ok", true));
                } else {
                    notifyListeners("ai:error", new JSObject().put("error", e.getMessage()));
                }
                call.resolve(new JSObject().put("ok", false));
            } finally {
                currentChatConnection = null;
            }
        }).start();
    }

    private void processSseData(String data, String provider) {
        try {
            JSONObject json = new JSONObject(data);

            if ("anthropic".equals(provider)) {
                String type = json.optString("type", "");
                if ("content_block_delta".equals(type)) {
                    JSONObject delta = json.optJSONObject("delta");
                    if (delta != null && "text_delta".equals(delta.optString("type"))) {
                        String text = delta.optString("text", "");
                        if (!text.isEmpty()) {
                            notifyListeners("ai:chunk", new JSObject().put("content", text));
                        }
                    }
                }
            } else {
                // OpenRouter / 智谱 / OpenAI 兼容格式
                JSONArray choices = json.optJSONArray("choices");
                if (choices != null && choices.length() > 0) {
                    JSONObject choice = choices.getJSONObject(0);
                    JSONObject delta = choice.optJSONObject("delta");
                    if (delta != null) {
                        String content = delta.optString("content", "");
                        if (!content.isEmpty()) {
                            notifyListeners("ai:chunk", new JSObject().put("content", content));
                        }
                    }
                }
            }
        } catch (Exception ignored) {}
    }

    private String readErrorStream(HttpURLConnection conn) {
        try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(conn.getErrorStream(), StandardCharsets.UTF_8))) {
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = reader.readLine()) != null) {
                sb.append(line);
            }
            return sb.toString();
        } catch (Exception e) {
            return "请求失败";
        }
    }

    @PluginMethod
    public void aiStop(PluginCall call) {
        try {
            if (currentChatConnection != null) {
                currentChatConnection.disconnect();
                currentChatConnection = null;
            }
            call.resolve(new JSObject().put("ok", true));
        } catch (Exception e) {
            call.reject("停止对话失败: " + e.getMessage());
        }
    }

    @PluginMethod
    public void aiGetModels(PluginCall call) {
        String provider = call.getString("provider", "");
        String apiKey = call.getString("api_key", "");
        String baseUrl = call.getString("base_url", "");

        new Thread(() -> {
            try {
                String apiUrl;
                switch (provider) {
                    case "anthropic":
                        apiUrl = baseUrl.isEmpty()
                                ? "https://api.anthropic.com/v1/models"
                                : baseUrl + "/v1/models";
                        break;
                    case "openrouter":
                        apiUrl = baseUrl.isEmpty()
                                ? "https://openrouter.ai/api/v1/models"
                                : baseUrl + "/v1/models";
                        break;
                    case "zhipu":
                        apiUrl = baseUrl.isEmpty()
                                ? "https://open.bigmodel.cn/api/paas/v4/models"
                                : baseUrl + "/v1/models";
                        break;
                    default:
                        apiUrl = baseUrl.isEmpty()
                                ? "https://openrouter.ai/api/v1/models"
                                : baseUrl + "/v1/models";
                        break;
                }

                URL url = new URL(apiUrl);
                HttpURLConnection conn = (HttpURLConnection) url.openConnection();
                conn.setRequestMethod("GET");
                conn.setConnectTimeout(15000);
                conn.setReadTimeout(30000);

                if ("anthropic".equals(provider)) {
                    conn.setRequestProperty("x-api-key", apiKey);
                    conn.setRequestProperty("anthropic-version", "2023-06-01");
                } else {
                    conn.setRequestProperty("Authorization", "Bearer " + apiKey);
                }

                int code = conn.getResponseCode();
                if (code != 200) {
                    call.reject("获取模型列表失败: " + code);
                    return;
                }

                StringBuilder sb = new StringBuilder();
                try (BufferedReader reader = new BufferedReader(
                        new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8))) {
                    String line;
                    while ((line = reader.readLine()) != null) {
                        sb.append(line);
                    }
                }

                call.resolve(new JSObject().put("models", sb.toString()));
            } catch (Exception e) {
                call.reject("获取模型列表失败: " + e.getMessage());
            }
        }).start();
    }

    @PluginMethod
    public void aiCheckProvider(PluginCall call) {
        String provider = call.getString("provider", "");
        String apiKey = call.getString("api_key", "");
        String baseUrl = call.getString("base_url", "");

        new Thread(() -> {
            try {
                String apiUrl;
                switch (provider) {
                    case "anthropic":
                        apiUrl = baseUrl.isEmpty()
                                ? "https://api.anthropic.com/v1/models"
                                : baseUrl + "/v1/models";
                        break;
                    case "openrouter":
                        apiUrl = baseUrl.isEmpty()
                                ? "https://openrouter.ai/api/v1/models"
                                : baseUrl + "/v1/models";
                        break;
                    case "zhipu":
                        apiUrl = baseUrl.isEmpty()
                                ? "https://open.bigmodel.cn/api/paas/v4/models"
                                : baseUrl + "/v1/models";
                        break;
                    default:
                        apiUrl = baseUrl.isEmpty()
                                ? "https://openrouter.ai/api/v1/models"
                                : baseUrl + "/v1/models";
                        break;
                }

                URL url = new URL(apiUrl);
                HttpURLConnection conn = (HttpURLConnection) url.openConnection();
                conn.setRequestMethod("GET");
                conn.setConnectTimeout(10000);
                conn.setReadTimeout(15000);

                if ("anthropic".equals(provider)) {
                    conn.setRequestProperty("x-api-key", apiKey);
                    conn.setRequestProperty("anthropic-version", "2023-06-01");
                } else {
                    conn.setRequestProperty("Authorization", "Bearer " + apiKey);
                }

                int code = conn.getResponseCode();
                JSObject ret = new JSObject();
                ret.put("ok", code == 200);
                if (code != 200) {
                    ret.put("error", "验证失败: HTTP " + code);
                }
                call.resolve(ret);
            } catch (Exception e) {
                JSObject ret = new JSObject();
                ret.put("ok", false);
                ret.put("error", e.getMessage());
                call.resolve(ret);
            }
        }).start();
    }

    // ==================== 项目管理 ====================

    @PluginMethod
    public void projectList(PluginCall call) {
        try {
            String projectsJson = getPrefs().getString(PROJECTS_KEY, "[]");
            call.resolve(new JSObject().put("projects", projectsJson));
        } catch (Exception e) {
            call.reject("获取项目列表失败: " + e.getMessage());
        }
    }

    @PluginMethod
    public void projectCreate(PluginCall call) {
        try {
            String name = call.getString("name", "");
            String workspacePath = call.getString("workspace_path", "");

            if (name.isEmpty()) {
                call.reject("项目名称不能为空");
                return;
            }

            String projectsJson = getPrefs().getString(PROJECTS_KEY, "[]");
            JSONArray projects = new JSONArray(projectsJson);

            String projectId = UUID.randomUUID().toString();
            long now = System.currentTimeMillis();

            JSONObject project = new JSONObject();
            project.put("id", projectId);
            project.put("name", name);
            project.put("path", workspacePath);
            project.put("createdAt", now);

            projects.put(project);

            getPrefs().edit().putString(PROJECTS_KEY, projects.toString()).apply();

            call.resolve(new JSObject().put("project", project.toString()));
        } catch (Exception e) {
            call.reject("创建项目失败: " + e.getMessage());
        }
    }

    @PluginMethod
    public void projectSwitch(PluginCall call) {
        try {
            String id = call.getString("id", "");
            if (id.isEmpty()) {
                call.reject("项目 ID 不能为空");
                return;
            }

            String projectsJson = getPrefs().getString(PROJECTS_KEY, "[]");
            JSONArray projects = new JSONArray(projectsJson);
            boolean found = false;

            for (int i = 0; i < projects.length(); i++) {
                if (id.equals(projects.getJSONObject(i).optString("id"))) {
                    found = true;
                    break;
                }
            }

            if (!found) {
                call.reject("项目不存在");
                return;
            }

            getPrefs().edit().putString(ACTIVE_PROJECT_KEY, id).apply();
            call.resolve(new JSObject().put("ok", true));
        } catch (Exception e) {
            call.reject("切换项目失败: " + e.getMessage());
        }
    }

    @PluginMethod
    public void projectDelete(PluginCall call) {
        try {
            String id = call.getString("id", "");
            if (id.isEmpty()) {
                call.reject("项目 ID 不能为空");
                return;
            }

            String projectsJson = getPrefs().getString(PROJECTS_KEY, "[]");
            JSONArray projects = new JSONArray(projectsJson);
            JSONArray newProjects = new JSONArray();

            for (int i = 0; i < projects.length(); i++) {
                JSONObject p = projects.getJSONObject(i);
                if (!id.equals(p.optString("id"))) {
                    newProjects.put(p);
                }
            }

            // 删除该项目的会话数据
            getPrefs().edit()
                    .putString(PROJECTS_KEY, newProjects.toString())
                    .remove(CONVERSATIONS_KEY_PREFIX + id)
                    .apply();

            // 如果删除的是当前活跃项目，清除活跃标记
            String activeId = getPrefs().getString(ACTIVE_PROJECT_KEY, "");
            if (id.equals(activeId)) {
                getPrefs().edit().remove(ACTIVE_PROJECT_KEY).apply();
            }

            call.resolve(new JSObject().put("ok", true));
        } catch (Exception e) {
            call.reject("删除项目失败: " + e.getMessage());
        }
    }

    // ==================== 会话管理 ====================

    @PluginMethod
    public void conversationList(PluginCall call) {
        try {
            String projectId = call.getString("project_id", "");
            if (projectId.isEmpty()) {
                call.reject("项目 ID 不能为空");
                return;
            }

            String key = CONVERSATIONS_KEY_PREFIX + projectId;
            String conversationsJson = getPrefs().getString(key, "[]");
            call.resolve(new JSObject().put("conversations", conversationsJson));
        } catch (Exception e) {
            call.reject("获取会话列表失败: " + e.getMessage());
        }
    }

    @PluginMethod
    public void conversationSave(PluginCall call) {
        try {
            String projectId = call.getString("project_id", "");
            String sessionId = call.getString("session_id", "");
            String messagesJson = call.getString("messages", "[]");
            String title = call.getString("title", "");

            if (projectId.isEmpty() || sessionId.isEmpty()) {
                call.reject("项目 ID 和会话 ID 不能为空");
                return;
            }

            String key = CONVERSATIONS_KEY_PREFIX + projectId;
            String conversationsJson = getPrefs().getString(key, "[]");
            JSONArray conversations = new JSONArray(conversationsJson);

            boolean updated = false;
            for (int i = 0; i < conversations.length(); i++) {
                JSONObject conv = conversations.getJSONObject(i);
                if (sessionId.equals(conv.optString("id"))) {
                    conv.put("messages", new JSONArray(messagesJson));
                    conv.put("updatedAt", System.currentTimeMillis());
                    if (!title.isEmpty()) {
                        conv.put("title", title);
                    }
                    updated = true;
                    break;
                }
            }

            if (!updated) {
                JSONObject newConv = new JSONObject();
                newConv.put("id", sessionId);
                newConv.put("projectId", projectId);
                newConv.put("title", title.isEmpty() ? "新会话" : title);
                newConv.put("messages", new JSONArray(messagesJson));
                newConv.put("createdAt", System.currentTimeMillis());
                newConv.put("updatedAt", System.currentTimeMillis());
                conversations.put(newConv);
            }

            getPrefs().edit().putString(key, conversations.toString()).apply();
            call.resolve(new JSObject().put("ok", true));
        } catch (Exception e) {
            call.reject("保存会话失败: " + e.getMessage());
        }
    }

    @PluginMethod
    public void conversationDelete(PluginCall call) {
        try {
            String projectId = call.getString("project_id", "");
            String sessionId = call.getString("session_id", "");

            if (projectId.isEmpty() || sessionId.isEmpty()) {
                call.reject("项目 ID 和会话 ID 不能为空");
                return;
            }

            String key = CONVERSATIONS_KEY_PREFIX + projectId;
            String conversationsJson = getPrefs().getString(key, "[]");
            JSONArray conversations = new JSONArray(conversationsJson);
            JSONArray newConversations = new JSONArray();

            for (int i = 0; i < conversations.length(); i++) {
                JSONObject conv = conversations.getJSONObject(i);
                if (!sessionId.equals(conv.optString("id"))) {
                    newConversations.put(conv);
                }
            }

            getPrefs().edit().putString(key, newConversations.toString()).apply();
            call.resolve(new JSObject().put("ok", true));
        } catch (Exception e) {
            call.reject("删除会话失败: " + e.getMessage());
        }
    }

    // ==================== 安全存储 ====================

    @PluginMethod
    public void secureGet(PluginCall call) {
        try {
            String key = call.getString("key", "");
            if (key.isEmpty()) {
                call.reject("Key 不能为空");
                return;
            }

            String encryptedValue = getSecurePrefs().getString(ENCRYPTED_KEY_PREFIX + key, null);
            JSObject ret = new JSObject();

            if (encryptedValue == null) {
                ret.put("value", JSONObject.NULL);
            } else {
                String decrypted = decrypt(encryptedValue);
                ret.put("value", decrypted);
            }

            call.resolve(ret);
        } catch (Exception e) {
            // 解密失败时返回 null
            JSObject ret = new JSObject();
            ret.put("value", JSONObject.NULL);
            call.resolve(ret);
        }
    }

    @PluginMethod
    public void secureSet(PluginCall call) {
        try {
            String key = call.getString("key", "");
            String value = call.getString("value", "");

            if (key.isEmpty()) {
                call.reject("Key 不能为空");
                return;
            }

            String encrypted = encrypt(value);
            getSecurePrefs().edit()
                    .putString(ENCRYPTED_KEY_PREFIX + key, encrypted)
                    .apply();

            call.resolve(new JSObject().put("ok", true));
        } catch (Exception e) {
            call.reject("安全存储失败: " + e.getMessage());
        }
    }

    @PluginMethod
    public void secureDelete(PluginCall call) {
        try {
            String key = call.getString("key", "");
            if (key.isEmpty()) {
                call.reject("Key 不能为空");
                return;
            }

            getSecurePrefs().edit()
                    .remove(ENCRYPTED_KEY_PREFIX + key)
                    .apply();

            call.resolve(new JSObject().put("ok", true));
        } catch (Exception e) {
            call.reject("删除失败: " + e.getMessage());
        }
    }

    // ==================== 系统信息 ====================

    @PluginMethod
    public void systemGetInfo(PluginCall call) {
        try {
            String appVersion = "1.0.0";
            try {
                PackageInfo pInfo = getContext().getPackageManager()
                        .getPackageInfo(getContext().getPackageName(), 0);
                appVersion = pInfo.versionName;
            } catch (Exception ignored) {}

            JSObject ret = new JSObject();
            ret.put("version", appVersion);
            ret.put("platform", "android");
            ret.put("device", Build.MODEL + " (Android " + Build.VERSION.RELEASE + ")");
            call.resolve(ret);
        } catch (Exception e) {
            call.reject("获取系统信息失败: " + e.getMessage());
        }
    }

    // ==================== 加密工具 ====================

    private SecretKey getOrCreateSecretKey() throws Exception {
        KeyStore keyStore = KeyStore.getInstance("AndroidKeyStore");
        keyStore.load(null);

        if (keyStore.containsAlias(ANDROID_KEYSTORE_ALIAS)) {
            KeyStore.SecretKeyEntry entry =
                    (KeyStore.SecretKeyEntry) keyStore.getEntry(ANDROID_KEYSTORE_ALIAS, null);
            return entry.getSecretKey();
        }

        KeyGenerator keyGenerator = KeyGenerator.getInstance(
                KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
        keyGenerator.init(
                new KeyGenParameterSpec.Builder(
                        ANDROID_KEYSTORE_ALIAS,
                        KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                        .setKeySize(256)
                        .build());
        return keyGenerator.generateKey();
    }

    private String encrypt(String plainText) throws Exception {
        SecretKey secretKey = getOrCreateSecretKey();
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.ENCRYPT_MODE, secretKey);
        byte[] iv = cipher.getIV();
        byte[] encrypted = cipher.doFinal(plainText.getBytes(StandardCharsets.UTF_8));

        // 格式: Base64(iv长度:iv:密文)
        byte[] combined = new byte[1 + iv.length + encrypted.length];
        combined[0] = (byte) iv.length;
        System.arraycopy(iv, 0, combined, 1, iv.length);
        System.arraycopy(encrypted, 0, combined, 1 + iv.length, encrypted.length);

        return Base64.encodeToString(combined, Base64.NO_WRAP);
    }

    private String decrypt(String encryptedText) throws Exception {
        SecretKey secretKey = getOrCreateSecretKey();
        byte[] combined = Base64.decode(encryptedText, Base64.NO_WRAP);

        int ivLength = combined[0] & 0xFF;
        byte[] iv = new byte[ivLength];
        byte[] encrypted = new byte[combined.length - 1 - ivLength];
        System.arraycopy(combined, 1, iv, 0, ivLength);
        System.arraycopy(combined, 1 + ivLength, encrypted, 0, encrypted.length);

        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        GCMParameterSpec spec = new GCMParameterSpec(128, iv);
        cipher.init(Cipher.DECRYPT_MODE, secretKey, spec);

        return new String(cipher.doFinal(encrypted), StandardCharsets.UTF_8);
    }
}
