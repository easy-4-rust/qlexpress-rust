package com.easy4rust.qlexpress;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

import com.alibaba.fastjson2.JSON;
import com.alibaba.fastjson2.JSONWriter;
import com.alibaba.fastjson2.JSONObject;
import com.alibaba.qlexpress4.aparser.InterpolationMode;
import com.alibaba.qlexpress4.aparser.ParserOperatorManager;
import com.alibaba.qlexpress4.aparser.QLexer;
import com.alibaba.qlexpress4.aparser.Token;
import com.alibaba.qlexpress4.exception.QLException;

/**
 * Java QLexer 的逐 Token 差分执行器。
 */
public final class JavaLexerDifferentialRunner {

    private JavaLexerDifferentialRunner() {
    }

    /**
     * 执行 JSONL 词法语料并输出稳定的逐 Token 记录。
     *
     * @param args 输入语料和输出记录路径
     * @throws IOException 文件读写失败
     */
    public static void main(String[] args)
        throws IOException {
        if (args.length != 2) {
            throw new IllegalArgumentException(
                "usage: JavaLexerDifferentialRunner <corpus.jsonl> <output.jsonl>");
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
                writer.write(JSON.toJSONString(execute(testCase), JSONWriter.Feature.WriteNulls));
                writer.newLine();
                count++;
            }
        }
        System.err.println("java lexer differential cases completed: " + count);
    }

    private static Map<String, Object> execute(JSONObject testCase) {
        String id = testCase.getString("id");
        String script = testCase.getString("script");
        String modeName = testCase.getString("interpolation_mode");
        InterpolationMode mode =
            modeName == null ? InterpolationMode.SCRIPT : InterpolationMode.valueOf(modeName);
        String selectorStart = testCase.getString("selector_start");
        String selectorEnd = testCase.getString("selector_end");
        boolean strictNewLines =
            !testCase.containsKey("strict_new_lines") || testCase.getBooleanValue("strict_new_lines");
        List<String> aliases = testCase.getList("aliases", String.class);
        ParserOperatorManager manager =
            aliases == null || aliases.isEmpty() ? null : new AliasManager(new HashSet<>(aliases));

        Map<String, Object> record = new LinkedHashMap<>();
        record.put("id", id);
        try {
            List<Map<String, Object>> tokenRecords = new ArrayList<>();
            for (Token token : QLexer.tokenize(script,
                manager,
                mode,
                selectorStart == null ? "${" : selectorStart,
                selectorEnd == null ? "}" : selectorEnd,
                strictNewLines)) {
                Map<String, Object> item = new LinkedHashMap<>();
                item.put("type", token.getType());
                item.put("text", normalizeUtf16(token.getText()));
                item.put("start", token.getStartIndex());
                item.put("stop", token.getStopIndex());
                item.put("line", token.getLine());
                item.put("column", token.getCharPositionInLine());
                tokenRecords.add(item);
            }
            record.put("outcome", "ok");
            record.put("tokens", tokenRecords);
            record.put("error_code", null);
            record.put("line", null);
            record.put("column", null);
            record.put("reason", null);
        }
        catch (QLException error) {
            record.put("outcome", "error");
            record.put("tokens", Collections.emptyList());
            record.put("error_code", error.getErrorCode());
            record.put("line", error.getLineNo());
            record.put("column", error.getColNo());
            record.put("reason", error.getReason());
        }
        return record;
    }

    /**
     * Rust String 不能表示未配对 UTF-16 代理项；Java oracle 将这类单元规范化
     * 为 U+FFFD，合法代理项对保持原字符。
     */
    private static String normalizeUtf16(String value) {
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
                result.append('\uFFFD');
            }
            else {
                result.append(current);
            }
        }
        return result.toString();
    }

    private static final class AliasManager
        implements ParserOperatorManager {

        private final Set<String> aliases;

        private AliasManager(Set<String> aliases) {
            this.aliases = aliases;
        }

        @Override
        public boolean isOpType(String lexeme, OpType opType) {
            return false;
        }

        @Override
        public Integer precedence(String lexeme) {
            return null;
        }

        @Override
        public Integer getAlias(String lexeme) {
            return aliases.contains(lexeme) ? QLexer.OPID : null;
        }
    }
}
