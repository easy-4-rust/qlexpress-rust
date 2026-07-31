package com.easy4rust.qlexpress;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.lang.reflect.Array;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import com.alibaba.fastjson2.JSON;
import com.alibaba.fastjson2.JSONWriter;
import com.alibaba.fastjson2.JSONObject;
import com.alibaba.qlexpress4.Express4Runner;
import com.alibaba.qlexpress4.InitOptions;
import com.alibaba.qlexpress4.QLOptions;
import com.alibaba.qlexpress4.QLResult;
import com.alibaba.qlexpress4.exception.QLException;
import com.alibaba.qlexpress4.security.QLSecurityStrategy;

/**
 * Java 4.2.0-beta 差分执行器。输出格式与 Rust verification crate 一致。
 */
public final class JavaDifferentialRunner {

    private JavaDifferentialRunner() {
    }

    public static void main(String[] args)
        throws IOException {
        if (args.length != 2) {
            throw new IllegalArgumentException("usage: JavaDifferentialRunner <corpus.jsonl> <output.jsonl>");
        }
        Path corpus = Paths.get(args[0]);
        Path output = Paths.get(args[1]);
        int count = 0;
        try (BufferedReader reader = Files.newBufferedReader(corpus, StandardCharsets.UTF_8);
            BufferedWriter writer = Files.newBufferedWriter(output, StandardCharsets.UTF_8)) {
            String line;
            while ((line = reader.readLine()) != null) {
                if (line.trim().isEmpty() || line.trim().startsWith("#")) {
                    continue;
                }
                JSONObject testCase = JSON.parseObject(line);
                Map<String, Object> record = execute(testCase);
                writer.write(JSON.toJSONString(record, JSONWriter.Feature.WriteNulls));
                writer.newLine();
                count++;
            }
        }
        System.err.println("java differential cases completed: " + count);
    }

    private static Map<String, Object> execute(JSONObject testCase) {
        String id = testCase.getString("id");
        String script = testCase.getString("script");
        JSONObject contextObject = testCase.getJSONObject("context");
        Map<String, Object> context =
            contextObject == null ? Collections.emptyMap() : new LinkedHashMap<>(contextObject);
        JSONObject optionObject = testCase.getJSONObject("options");
        InitOptions initOptions = InitOptions.builder()
            .securityStrategy(QLSecurityStrategy.open())
            .traceExpression(true)
            .build();
        Express4Runner runner = new Express4Runner(initOptions);
        QLOptions options = options(optionObject);
        Map<String, Object> record = new LinkedHashMap<>();
        record.put("id", id);
        try {
            QLResult result = runner.execute(script, context, options);
            record.put("outcome", "ok");
            record.put("normalized", normalize(result.getResult()));
            record.put("error_code", null);
            record.put("line", null);
            record.put("column", null);
            record.put("trace_count",
                result.getExpressionTraces() == null ? 0 : result.getExpressionTraces().size());
        }
        catch (QLException error) {
            record.put("outcome", "error");
            record.put("normalized", null);
            record.put("error_code", error.getErrorCode());
            record.put("line", error.getLineNo());
            record.put("column", error.getColNo());
            record.put("trace_count", 0);
        }
        return record;
    }

    private static QLOptions options(JSONObject options) {
        QLOptions.Builder builder = QLOptions.builder();
        if (options == null) {
            return builder.build();
        }
        if (options.containsKey("precise")) {
            builder.precise(options.getBooleanValue("precise"));
        }
        if (options.containsKey("cache")) {
            builder.cache(options.getBooleanValue("cache"));
        }
        if (options.containsKey("avoid_null_pointer")) {
            builder.avoidNullPointer(options.getBooleanValue("avoid_null_pointer"));
        }
        if (options.containsKey("max_arr_length")) {
            builder.maxArrLength(options.getIntValue("max_arr_length"));
        }
        if (options.containsKey("trace_expression")) {
            builder.traceExpression(options.getBooleanValue("trace_expression"));
        }
        if (options.containsKey("short_circuit_disable")) {
            builder.shortCircuitDisable(options.getBooleanValue("short_circuit_disable"));
        }
        if (options.containsKey("timeout_millis")) {
            builder.timeoutMillis(options.getLongValue("timeout_millis"));
        }
        return builder.build();
    }

    private static String normalize(Object value) {
        if (value == null) {
            return "null";
        }
        if (value instanceof Boolean) {
            return "bool:" + value;
        }
        if (value instanceof Byte) {
            return "byte:" + value;
        }
        if (value instanceof Short) {
            return "short:" + value;
        }
        if (value instanceof Integer) {
            return "int:" + value;
        }
        if (value instanceof Long) {
            return "long:" + value;
        }
        if (value instanceof Float) {
            return "float:" + normalizeFloating(((Float)value).doubleValue());
        }
        if (value instanceof Double) {
            return "double:" + normalizeFloating((Double)value);
        }
        if (value instanceof BigInteger) {
            return "bigint:" + value;
        }
        if (value instanceof BigDecimal) {
            return "bigdec:" + ((BigDecimal)value).toPlainString();
        }
        if (value instanceof Character) {
            return "char:" + escapeUtf16(value.toString());
        }
        if (value instanceof String) {
            return "string:" + escapeUtf16((String)value);
        }
        if (value instanceof List<?>) {
            List<String> values = new ArrayList<>();
            for (Object item : (List<?>)value) {
                values.add(normalize(item));
            }
            return "list:[" + String.join(",", values) + "]";
        }
        if (value instanceof Map<?, ?>) {
            List<Map.Entry<?, ?>> entries = new ArrayList<>(((Map<?, ?>)value).entrySet());
            if (!(value instanceof LinkedHashMap<?, ?>)) {
                entries.sort(Comparator.comparing(entry -> normalize(entry.getKey())));
            }
            List<String> values = new ArrayList<>();
            for (Map.Entry<?, ?> entry : entries) {
                values.add(normalize(entry.getKey()) + "=>" + normalize(entry.getValue()));
            }
            return "map:{" + String.join(",", values) + "}";
        }
        if (value.getClass().isArray()) {
            List<String> values = new ArrayList<>();
            for (int i = 0; i < Array.getLength(value); i++) {
                values.add(normalize(Array.get(value, i)));
            }
            return "array:[" + String.join(",", values) + "]";
        }
        return "object:" + value.getClass().getName();
    }

    private static String normalizeFloating(double value) {
        if (Double.isNaN(value)) {
            return "NaN";
        }
        if (value == Double.POSITIVE_INFINITY) {
            return "Infinity";
        }
        if (value == Double.NEGATIVE_INFINITY) {
            return "-Infinity";
        }
        return Double.toString(value);
    }

    private static String escape(String value) {
        return value.replace("\\", "\\\\")
            .replace(":", "\\:")
            .replace(",", "\\,")
            .replace("[", "\\[")
            .replace("]", "\\]")
            .replace("{", "\\{")
            .replace("}", "\\}");
    }

    /**
     * 将未配对 UTF-16 代理项规范化为稳定的十六进制文本。
     *
     * <p>合法代理项对保持原字符；未配对代理项不能无损写入 UTF-8，
     * 因此显式写为“反斜杠-u-XXXX”，与 Rust 差分执行器一致。</p>
     *
     * @param value Java 字符串
     * @return 可安全写入 UTF-8 差分结果的文本
     */
    private static String escapeUtf16(String value) {
        StringBuilder result = new StringBuilder();
        for (int index = 0; index < value.length(); index++) {
            char current = value.charAt(index);
            if (Character.isHighSurrogate(current)
                && index + 1 < value.length()
                && Character.isLowSurrogate(value.charAt(index + 1))) {
                result.append(current);
                result.append(value.charAt(++index));
            }
            else if (Character.isSurrogate(current)) {
                result.append(String.format("\\u%04X", (int)current));
            }
            else {
                result.append(current);
            }
        }
        return escape(result.toString());
    }
}
