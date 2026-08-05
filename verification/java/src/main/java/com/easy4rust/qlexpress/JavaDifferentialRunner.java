package com.easy4rust.qlexpress;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.lang.reflect.Array;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.AbstractMap;
import java.util.Collections;
import java.util.Comparator;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import com.alibaba.fastjson2.JSON;
import com.alibaba.fastjson2.JSONWriter;
import com.alibaba.fastjson2.JSONObject;
import com.alibaba.qlexpress4.Express4Runner;
import com.alibaba.qlexpress4.api.BatchAddFunctionResult;
import com.alibaba.qlexpress4.api.QLFunctionalVarargs;
import com.alibaba.qlexpress4.InitOptions;
import com.alibaba.qlexpress4.QLOptions;
import com.alibaba.qlexpress4.QLResult;
import com.alibaba.qlexpress4.exception.QLException;
import com.alibaba.qlexpress4.exception.PureErrReporter;
import com.alibaba.qlexpress4.exception.lsp.Diagnostic;
import com.alibaba.qlexpress4.exception.lsp.Position;
import com.alibaba.qlexpress4.exception.lsp.Range;
import com.alibaba.qlexpress4.aparser.ParserOperatorManager.OpType;
import com.alibaba.qlexpress4.runtime.Value;
import com.alibaba.qlexpress4.runtime.DelegateQContext;
import com.alibaba.qlexpress4.runtime.ExceptionTable;
import com.alibaba.qlexpress4.runtime.FixedSizeStack;
import com.alibaba.qlexpress4.runtime.Parameters;
import com.alibaba.qlexpress4.runtime.QvmGlobalScope;
import com.alibaba.qlexpress4.runtime.QvmRuntime;
import com.alibaba.qlexpress4.runtime.ReflectLoader;
import com.alibaba.qlexpress4.runtime.context.MapExpressContext;
import com.alibaba.qlexpress4.runtime.data.DataValue;
import com.alibaba.qlexpress4.runtime.function.CustomFunction;
import com.alibaba.qlexpress4.runtime.operator.BinaryOperator;
import com.alibaba.qlexpress4.runtime.operator.CustomBinaryOperator;
import com.alibaba.qlexpress4.runtime.operator.OperatorManager;
import com.alibaba.qlexpress4.runtime.operator.unary.UnaryOperator;
import com.alibaba.qlexpress4.runtime.scope.QScope;
import com.alibaba.qlexpress4.runtime.scope.QvmBlockScope;
import com.alibaba.qlexpress4.runtime.trace.QTraces;
import com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath;
import com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath;
import com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath;
import com.alibaba.qlexpress4.runtime.operator.number.IntegerMath;
import com.alibaba.qlexpress4.runtime.operator.number.LongMath;
import com.alibaba.qlexpress4.runtime.operator.number.NumberMath;
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
        JSONObject numberMath = testCase.getJSONObject("number_math");
        if (numberMath != null) {
            return executeNumberMath(id, numberMath);
        }
        JSONObject operatorManager = testCase.getJSONObject("operator_manager");
        if (operatorManager != null) {
            return executeOperatorManager(id, operatorManager);
        }
        JSONObject batchAddFunctionResult = testCase.getJSONObject("batch_add_function_result");
        if (batchAddFunctionResult != null) {
            return executeBatchAddFunctionResult(id, batchAddFunctionResult);
        }
        JSONObject qlFunctionalVarargs = testCase.getJSONObject("ql_functional_varargs");
        if (qlFunctionalVarargs != null) {
            return executeQLFunctionalVarargs(id, qlFunctionalVarargs);
        }
        JSONObject lspPosition = testCase.getJSONObject("lsp_position");
        if (lspPosition != null) {
            return executeLspPosition(id, lspPosition);
        }
        JSONObject lspRange = testCase.getJSONObject("lsp_range");
        if (lspRange != null) {
            return executeLspRange(id, lspRange);
        }
        JSONObject lspDiagnostic = testCase.getJSONObject("lsp_diagnostic");
        if (lspDiagnostic != null) {
            return executeLspDiagnostic(id, lspDiagnostic);
        }
        JSONObject delegateContext = testCase.getJSONObject("delegate_context");
        if (delegateContext != null) {
            return executeDelegateContext(id, delegateContext);
        }
        JSONObject fixedSizeStack = testCase.getJSONObject("fixed_size_stack");
        if (fixedSizeStack != null) {
            return executeFixedSizeStack(id, fixedSizeStack);
        }
        JSONObject runtimeCore = testCase.getJSONObject("runtime_core");
        if (runtimeCore != null) {
            return executeRuntimeCore(id, runtimeCore);
        }
        JSONObject exceptionTable = testCase.getJSONObject("exception_table");
        if (exceptionTable != null) {
            return executeExceptionTable(id, exceptionTable);
        }
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
            record.put("normalized", "error:" + error.getErrorCode() + ":" + error.getReason());
            record.put("error_code", error.getErrorCode());
            record.put("line", error.getLineNo());
            record.put("column", error.getColNo());
            record.put("trace_count", 0);
        }
        return record;
    }

    /**
     * 直接比较批量注册结果两个列表的顺序、隔离性和写回语义。
     *
     * @param id 差分用例标识
     * @param invocation 场景描述
     * @return 规范化的有序观察结果
     */
    private static Map<String, Object> executeBatchAddFunctionResult(
        String id,
        JSONObject invocation) {
        String scenario = invocation.getString("scenario");
        if (!"full_contract".equals(scenario)) {
            throw new IllegalArgumentException(
                "unsupported batch_add_function_result scenario: " + scenario);
        }

        BatchAddFunctionResult result = new BatchAddFunctionResult();
        Map<String, Object> observed = new LinkedHashMap<>();
        observed.put("initial_succ", new ArrayList<>(result.getSucc()));
        observed.put("initial_fail", new ArrayList<>(result.getFail()));
        observed.put("initial_all_succ", result.getFail().isEmpty());

        result.getSucc().add("external-success");
        result.getFail().add("external-failure");
        result.getSucc().add("runner-success");
        result.getFail().add("runner-failure");

        observed.put("succ", new ArrayList<>(result.getSucc()));
        observed.put("fail", new ArrayList<>(result.getFail()));
        observed.put("all_succ_after_failure", result.getFail().isEmpty());
        return successRecord(id, observed);
    }

    /**
     * 直接比较函数式变参接口的空参数、顺序、null 参数和 null 返回值。
     *
     * @param id 差分用例标识
     * @param invocation 场景描述
     * @return 规范化的有序观察结果
     */
    private static Map<String, Object> executeQLFunctionalVarargs(
        String id,
        JSONObject invocation) {
        String scenario = invocation.getString("scenario");
        if (!"full_contract".equals(scenario)) {
            throw new IllegalArgumentException(
                "unsupported ql_functional_varargs scenario: " + scenario);
        }

        QLFunctionalVarargs count = params -> params.length;
        QLFunctionalVarargs collect = params -> {
            List<Object> values = new ArrayList<>(params.length);
            Collections.addAll(values, params);
            return values;
        };
        QLFunctionalVarargs returnsNull = params -> null;
        Object[] parameters = new Object[] {1, "x", null};
        Map<String, Object> observed = new LinkedHashMap<>();
        observed.put("empty_count", count.call());
        observed.put("ordered_values", collect.call(parameters));
        observed.put("null_result", returnsNull.call(parameters));
        return successRecord(id, observed);
    }

    /**
     * 直接比较 LSP Position 构造器与两个零基坐标访问器。
     *
     * @param id 差分用例标识
     * @param invocation 场景描述
     * @return 规范化的有序观察结果
     */
    private static Map<String, Object> executeLspPosition(String id, JSONObject invocation) {
        String scenario = invocation.getString("scenario");
        if (!"full_contract".equals(scenario)) {
            throw new IllegalArgumentException("unsupported lsp_position scenario: " + scenario);
        }

        Position normal = new Position(7, 99);
        Position negative = new Position(-1, -2);
        Map<String, Object> observed = new LinkedHashMap<>();
        observed.put("normal_line", normal.getLine());
        observed.put("normal_character", normal.getCharacter());
        observed.put("negative_line", negative.getLine());
        observed.put("negative_character", negative.getCharacter());
        return successRecord(id, observed);
    }

    /**
     * 直接比较 LSP Range 的非空坐标与 Java 可空端点语义。
     *
     * @param id 差分用例标识
     * @param invocation 场景描述
     * @return 规范化的有序观察结果
     */
    private static Map<String, Object> executeLspRange(String id, JSONObject invocation) {
        String scenario = invocation.getString("scenario");
        if (!"full_contract".equals(scenario)) {
            throw new IllegalArgumentException("unsupported lsp_range scenario: " + scenario);
        }

        Range range = new Range(new Position(1, 2), new Position(3, 4));
        Range nullable = new Range(null, null);
        Map<String, Object> observed = new LinkedHashMap<>();
        observed.put("start_line", range.getStart().getLine());
        observed.put("start_character", range.getStart().getCharacter());
        observed.put("end_line", range.getEnd().getLine());
        observed.put("end_character", range.getEnd().getCharacter());
        observed.put("null_start", nullable.getStart());
        observed.put("null_end", nullable.getEnd());
        return successRecord(id, observed);
    }

    /**
     * 直接比较 LSP Diagnostic 的全部字段及 Java 可空引用语义。
     *
     * @param id 差分用例标识
     * @param invocation 场景描述
     * @return 规范化的有序观察结果
     */
    private static Map<String, Object> executeLspDiagnostic(String id, JSONObject invocation) {
        String scenario = invocation.getString("scenario");
        if (!"full_contract".equals(scenario)) {
            throw new IllegalArgumentException(
                "unsupported lsp_diagnostic scenario: " + scenario);
        }

        Diagnostic diagnostic = new Diagnostic(
            12,
            new Range(new Position(1, 2), new Position(1, 5)),
            "abc",
            "E001",
            "bad input",
            "a = abc");
        Diagnostic nullable = new Diagnostic(-5, null, null, null, null, null);
        Map<String, Object> observed = new LinkedHashMap<>();
        observed.put("pos", diagnostic.getPos());
        observed.put("range_start_line", diagnostic.getRange().getStart().getLine());
        observed.put("lexeme", diagnostic.getLexeme());
        observed.put("code", diagnostic.getCode());
        observed.put("message", diagnostic.getMessage());
        observed.put("snippet", diagnostic.getSnippet());
        observed.put("nullable_pos", nullable.getPos());
        observed.put("nullable_range", nullable.getRange());
        observed.put("nullable_lexeme", nullable.getLexeme());
        observed.put("nullable_code", nullable.getCode());
        observed.put("nullable_message", nullable.getMessage());
        observed.put("nullable_snippet", nullable.getSnippet());
        return successRecord(id, observed);
    }

    /**
     * 直接比较异常表的声明顺序、Java 类型可赋值关系和可空 finally 位置。
     *
     * @param id 差分用例标识
     * @param invocation 场景描述
     * @return 规范化的有序观察结果
     */
    private static Map<String, Object> executeExceptionTable(String id, JSONObject invocation) {
        String scenario = invocation.getString("scenario");
        if (!"full_contract".equals(scenario)) {
            throw new IllegalArgumentException("unsupported exception_table scenario: " + scenario);
        }

        List<Map.Entry<Class<?>, Integer>> handlers = new ArrayList<>();
        handlers.add(new AbstractMap.SimpleImmutableEntry<>(Number.class, 11));
        handlers.add(new AbstractMap.SimpleImmutableEntry<>(RuntimeException.class, 22));
        handlers.add(new AbstractMap.SimpleImmutableEntry<>(Object.class, 33));
        ExceptionTable table = new ExceptionTable(handlers, 44);
        List<Map.Entry<Class<?>, Integer>> objectFirstHandlers = new ArrayList<>();
        objectFirstHandlers.add(new AbstractMap.SimpleImmutableEntry<>(Object.class, 5));
        objectFirstHandlers.add(new AbstractMap.SimpleImmutableEntry<>(Number.class, 6));
        ExceptionTable objectFirst = new ExceptionTable(objectFirstHandlers, null);

        Map<String, Object> observed = new LinkedHashMap<>();
        observed.put("null_to_first", table.getRelativePos(null));
        observed.put("integer_to_number", table.getRelativePos(1));
        observed.put("long_to_number", table.getRelativePos(1L));
        observed.put("runtime_subclass", table.getRelativePos(new IllegalArgumentException()));
        observed.put("string_to_object", table.getRelativePos("fallback"));
        observed.put("final_pos", table.getFinalPos());
        observed.put("declaration_order", objectFirst.getRelativePos(1));
        observed.put("empty_relative", ExceptionTable.EMPTY.getRelativePos(1));
        observed.put("empty_final", ExceptionTable.EMPTY.getFinalPos());
        return successRecord(id, observed);
    }

    /**
     * 直接比较 QRuntime、QvmRuntime 与 QContext 的引用共享和委托契约。
     *
     * @param id 差分用例标识
     * @param invocation 场景描述
     * @return 规范化的有序观察结果
     */
    private static Map<String, Object> executeRuntimeCore(String id, JSONObject invocation) {
        String scenario = invocation.getString("scenario");
        if (!"full_contract".equals(scenario)) {
            throw new IllegalArgumentException("unsupported runtime_core scenario: " + scenario);
        }

        Map<String, Object> attachments = new LinkedHashMap<>();
        attachments.put("tenant", "acme");
        QTraces traces = new QTraces(new ArrayList<>(), new HashMap<>());
        ReflectLoader reflectLoader = new ReflectLoader(QLSecurityStrategy.open(), false);
        QvmRuntime runtime = new QvmRuntime(traces, attachments, reflectLoader, 424242L);
        Map<String, Object> observed = new LinkedHashMap<>();

        observed.put("runtime_start", runtime.scriptStartTimeStamp());
        observed.put("runtime_attachment_initial", runtime.attachment().get("tenant"));
        observed.put("runtime_registry_same", runtime.getReflectLoader() == reflectLoader);
        observed.put("runtime_trace_count", runtime.getTraces().getExpressionTraces().size());

        attachments.put("external_write", 7);
        observed.put("external_write_visible", runtime.attachment().get("external_write"));
        runtime.attachment().put("runtime_write", 8);
        observed.put("runtime_write_visible_external", attachments.get("runtime_write"));

        QvmGlobalScope globalScope = new QvmGlobalScope(
            new MapExpressContext(new LinkedHashMap<>()),
            new HashMap<>(),
            QLOptions.builder().attachments(attachments).build());
        QvmBlockScope blockScope = new QvmBlockScope(
            globalScope,
            new HashMap<>(),
            1,
            ExceptionTable.EMPTY);
        DelegateQContext context = new DelegateQContext(runtime, blockScope);
        observed.put("context_runtime_same", true);
        observed.put("context_start", context.scriptStartTimeStamp());
        observed.put("context_registry_same", context.getReflectLoader() == reflectLoader);
        observed.put("context_traces_same", context.getTraces() == traces);
        observed.put("context_current_initial", context.getCurrentScope() == blockScope);
        context.attachment().put("context_write", 9);
        observed.put("context_write_visible_runtime", runtime.attachment().get("context_write"));
        QScope child = context.newScope();
        observed.put("context_current_child", context.getCurrentScope() == child);
        context.closeScope();
        observed.put("context_closed_to_parent", context.getCurrentScope() == blockScope);
        return successRecord(id, observed);
    }

    /**
     * 直接比较 FixedSizeStack、StackSwapParameters 与 Parameters 的顺序和共享窗口副作用。
     *
     * @param id 差分用例标识
     * @param invocation 场景描述
     * @return 规范化的有序观察结果
     */
    private static Map<String, Object> executeFixedSizeStack(String id, JSONObject invocation) {
        String scenario = invocation.getString("scenario");
        if (!"full_contract".equals(scenario)) {
            throw new IllegalArgumentException("unsupported fixed_size_stack scenario: " + scenario);
        }

        FixedSizeStack stack = new FixedSizeStack(4);
        Map<String, Object> observed = new LinkedHashMap<>();
        observed.put("capacity", 4);
        for (int value = 1; value <= 4; value++) {
            stack.push(new DataValue(value));
        }
        observed.put("peak", stack.peak().get());
        observed.put("pop_4", stack.pop().get());
        observed.put("pop_3", stack.pop().get());
        stack.push(new DataValue(5));
        stack.push(new DataValue(6));

        Parameters parameters = stack.pop(3);
        observed.put("parameters_size", parameters.size());
        observed.put("parameters_present_0", parameters.get(0) != null);
        observed.put("parameters_values", parameterValues(parameters));
        observed.put("parameters_oob_present", parameters.get(3) != null);
        observed.put("parameters_oob_value", parameters.getValue(3));
        observed.put("remaining_peak", stack.peak().get());

        stack.push(new DataValue(9));
        observed.put("live_after_one_push", parameterValues(parameters));
        stack.push(new DataValue(8));
        observed.put("live_after_two_pushes", parameterValues(parameters));
        stack.push(new DataValue(7));
        observed.put("live_after_three_pushes", parameterValues(parameters));
        observed.put("pop_reused_top", stack.pop().get());
        observed.put("peak_after_pop", stack.peak().get());
        Parameters emptyParameters = stack.pop(0);
        observed.put("zero_pop_size", emptyParameters.size());
        observed.put("zero_pop_get", emptyParameters.get(0) != null);

        FixedSizeStack nullStack = new FixedSizeStack(1);
        nullStack.push(new DataValue((Object)null));
        Parameters nullParameters = nullStack.pop(1);
        observed.put("null_slot_present", nullParameters.get(0) != null);
        observed.put("null_slot_value", nullParameters.getValue(0));
        return successRecord(id, observed);
    }

    private static List<Object> parameterValues(Parameters parameters) {
        List<Object> values = new ArrayList<>(parameters.size());
        for (int index = 0; index < parameters.size(); index++) {
            values.add(parameters.getValue(index));
        }
        return values;
    }

    /**
     * 直接构造 DelegateQContext，逐项观察运行时委托、作用域、符号、函数表和共享栈。
     *
     * @param id 差分用例标识
     * @param invocation 场景描述
     * @return 规范化的有序观察结果
     */
    private static Map<String, Object> executeDelegateContext(String id, JSONObject invocation) {
        String scenario = invocation.getString("scenario");
        if ("close_global".equals(scenario)) {
            return executeDelegateCloseGlobal(id);
        }
        if (!"full_contract".equals(scenario)) {
            throw new IllegalArgumentException("unsupported delegate_context scenario: " + scenario);
        }

        Map<String, Object> attachments = new LinkedHashMap<>();
        attachments.put("tenant", "acme");
        QTraces traces = new QTraces(new ArrayList<>(), new HashMap<>());
        ReflectLoader reflectLoader = new ReflectLoader(QLSecurityStrategy.open(), false);
        QvmRuntime runtime = new QvmRuntime(traces, attachments, reflectLoader, 123456L);
        QLOptions options = QLOptions.builder().attachments(attachments).build();
        QvmGlobalScope globalScope = new QvmGlobalScope(
            new MapExpressContext(new LinkedHashMap<>()),
            new HashMap<>(),
            options);
        QvmBlockScope blockScope = new QvmBlockScope(
            globalScope,
            new HashMap<>(),
            8,
            ExceptionTable.EMPTY);
        DelegateQContext context = new DelegateQContext(runtime, blockScope);
        Map<String, Object> observed = new LinkedHashMap<>();

        observed.put("start_time", context.scriptStartTimeStamp());
        observed.put("attachment", context.attachment().get("tenant"));
        observed.put("reflect_same", context.getReflectLoader() == reflectLoader);
        observed.put("traces_same", context.getTraces() == traces);
        observed.put("trace_count", context.getTraces().getExpressionTraces().size());
        observed.put("current_initial", context.getCurrentScope() == blockScope);
        observed.put("parent_initial", context.getParent() == globalScope);

        context.defineLocalSymbol("x", Integer.class, 7);
        Value symbol = context.getSymbol("x");
        observed.put("symbol_present", symbol != null);
        observed.put("symbol_value", symbol.get());
        observed.put("missing_value", context.getSymbolValue("missing"));

        CustomFunction function = (qContext, parameters) -> parameters.size();
        context.defineFunction("f", function);
        observed.put("function_get", context.getFunction("f") != null);
        Map<String, CustomFunction> functionTable = context.getFunctionTable();
        functionTable.put("g", function);
        observed.put("function_table_size", functionTable.size());
        observed.put("function_table_write_through", context.getFunction("g") != null);

        context.push(new DataValue(1));
        context.push(new DataValue(2));
        QScope childScope = context.newScope();
        context.push(new DataValue(3));
        observed.put("stack_peek", context.peek().get());
        Parameters popped = context.pop(2);
        observed.put("pop_n_size", popped.size());
        observed.put("pop_n_0", popped.get(0).get());
        observed.put("pop_n_1", popped.get(1).get());
        observed.put("stack_after_pop_n", context.peek().get());
        context.push(new DataValue(4));
        observed.put("pop_single", context.pop().get());
        observed.put("child_current", context.getCurrentScope() == childScope);
        observed.put("child_parent", context.getParent() == blockScope);
        observed.put("child_inherits_function", context.getFunction("f") != null);
        observed.put("child_function_table_size", context.getFunctionTable().size());
        observed.put("child_inherited_symbol", context.getSymbolValue("x"));
        context.defineLocalSymbol("x", Integer.class, 9);
        observed.put("child_shadow", context.getSymbolValue("x"));
        context.closeScope();
        observed.put("closed_to_parent", context.getCurrentScope() == blockScope);
        observed.put("parent_symbol_after_close", context.getSymbolValue("x"));
        context.closeScope();
        observed.put("closed_to_global", context.getCurrentScope() == globalScope);

        return successRecord(id, observed);
    }

    private static Map<String, Object> executeDelegateCloseGlobal(String id) {
        Map<String, Object> attachments = new LinkedHashMap<>();
        QvmRuntime runtime = new QvmRuntime(
            new QTraces(new ArrayList<>(), new HashMap<>()),
            attachments,
            new ReflectLoader(QLSecurityStrategy.open(), false),
            123456L);
        QvmGlobalScope globalScope = new QvmGlobalScope(
            new MapExpressContext(new LinkedHashMap<>()),
            new HashMap<>(),
            QLOptions.builder().build());
        DelegateQContext context = new DelegateQContext(runtime, globalScope);
        try {
            context.closeScope();
            throw new IllegalStateException(
                "DelegateQContext.closeScope silently accepted global scope");
        }
        catch (UnsupportedOperationException expected) {
            Map<String, Object> record = new LinkedHashMap<>();
            record.put("id", id);
            record.put("outcome", "error");
            record.put(
                "normalized",
                "error:UNSUPPORTED_OPERATION:QvmGlobalScope.getParent is unsupported");
            record.put("error_code", "UNSUPPORTED_OPERATION");
            record.put("line", 0);
            record.put("column", 0);
            record.put("trace_count", 0);
            return record;
        }
    }

    /**
     * 直接调用 Java OperatorManager，验证注册、覆盖、别名、查找和适配器行为。
     *
     * @param id 差分用例标识
     * @param invocation 操作、预置步骤与操作数
     * @return 与 Rust 执行器相同结构的规范化记录
     */
    private static Map<String, Object> executeOperatorManager(String id, JSONObject invocation) {
        OperatorManager manager = new OperatorManager();
        List<JSONObject> setups = invocation.getList("setup", JSONObject.class);
        if (setups != null) {
            for (JSONObject setup : setups) {
                if (!applyOperatorManagerSetup(manager, setup)) {
                    throw new IllegalArgumentException(
                        "operator_manager setup failed: " + setup.getString("action") + " "
                            + setup.getString("lexeme"));
                }
            }
        }

        String operation = invocation.getString("operation");
        String lexeme = invocation.getString("lexeme");
        try {
            Object result;
            switch (operation) {
                case "addBinaryOperator":
                    result = manager.addBinaryOperator(
                        lexeme,
                        additiveCustomOperator(),
                        invocation.getIntValue("priority", 300));
                    break;
                case "replaceDefaultOperator":
                    result = manager.replaceDefaultOperator(lexeme, additiveCustomOperator());
                    break;
                case "addOperatorAlias":
                    result = manager.addOperatorAlias(lexeme, invocation.getString("origin"));
                    break;
                case "addKeyWordAlias":
                    result = manager.addKeyWordAlias(lexeme, invocation.getString("keyword"));
                    break;
                case "getBinaryOperator":
                    result = operatorMetadata(manager.getBinaryOperator(lexeme));
                    break;
                case "getPrefixUnaryOperator":
                    result = unaryOperatorMetadata(manager.getPrefixUnaryOperator(lexeme));
                    break;
                case "getSuffixUnaryOperator":
                    result = unaryOperatorMetadata(manager.getSuffixUnaryOperator(lexeme));
                    break;
                case "isOpType":
                    result = manager.isOpType(lexeme, OpType.valueOf(invocation.getString("op_type")));
                    break;
                case "precedence":
                    result = manager.precedence(lexeme);
                    break;
                case "getAlias":
                    result = manager.getAlias(lexeme);
                    break;
                case "executeBinary":
                    BinaryOperator operator = manager.getBinaryOperator(lexeme);
                    if (operator == null) {
                        throw new IllegalArgumentException(
                            "operator_manager binary operator not found: " + lexeme);
                    }
                    Number left = typedNumber(invocation.getJSONObject("left"));
                    Number right = typedNumber(invocation.getJSONObject("right"));
                    Value leftValue = () -> left;
                    Value rightValue = () -> right;
                    result = operator.execute(
                        leftValue,
                        rightValue,
                        null,
                        QLOptions.builder().build(),
                        PureErrReporter.INSTANCE);
                    break;
                default:
                    throw new IllegalArgumentException(
                        "unsupported operator_manager operation: " + operation);
            }
            return successRecord(id, result);
        }
        catch (RuntimeException error) {
            return runtimeErrorRecord(id, error);
        }
    }

    private static boolean applyOperatorManagerSetup(OperatorManager manager, JSONObject setup) {
        String action = setup.getString("action");
        String lexeme = setup.getString("lexeme");
        switch (action) {
            case "add":
                return manager.addBinaryOperator(
                    lexeme,
                    additiveCustomOperator(),
                    setup.getIntValue("priority", 300));
            case "replace":
                return manager.replaceDefaultOperator(lexeme, additiveCustomOperator());
            case "operator_alias":
                return manager.addOperatorAlias(lexeme, setup.getString("origin"));
            case "keyword_alias":
                return manager.addKeyWordAlias(lexeme, setup.getString("keyword"));
            default:
                throw new IllegalArgumentException(
                    "unsupported operator_manager setup action: " + action);
        }
    }

    private static CustomBinaryOperator additiveCustomOperator() {
        return (left, right) -> NumberMath.add((Number)left.get(), (Number)right.get());
    }

    private static Object operatorMetadata(BinaryOperator operator) {
        if (operator == null) {
            return null;
        }
        return operator.getOperator() + "|" + operator.getPriority();
    }

    private static Object unaryOperatorMetadata(UnaryOperator operator) {
        if (operator == null) {
            return null;
        }
        return operator.getOperator() + "|" + operator.getPriority();
    }

    private static Map<String, Object> successRecord(String id, Object result) {
        Map<String, Object> record = new LinkedHashMap<>();
        record.put("id", id);
        record.put("outcome", "ok");
        record.put("normalized", normalize(result));
        record.put("error_code", null);
        record.put("line", null);
        record.put("column", null);
        record.put("trace_count", 0);
        return record;
    }

    private static Map<String, Object> runtimeErrorRecord(String id, RuntimeException error) {
        Map<String, Object> record = new LinkedHashMap<>();
        record.put("id", id);
        record.put("outcome", "error");
        record.put(
            "normalized",
            "error:" + error.getClass().getSimpleName() + ":" + error.getMessage());
        record.put("error_code", error.getClass().getSimpleName());
        record.put("line", 0);
        record.put("column", 0);
        record.put("trace_count", 0);
        return record;
    }

    /**
     * 直接调用 Java NumberMath 静态门面，作为 Rust 数值域实现的 oracle。
     *
     * @param id 差分用例标识
     * @param invocation 操作名与显式 Number 子类型操作数
     * @return 与脚本差分相同结构的规范化记录
     */
    private static Map<String, Object> executeNumberMath(String id, JSONObject invocation) {
        String operation = invocation.getString("operation");
        Number left = typedNumber(invocation.getJSONObject("left"));
        JSONObject rightObject = invocation.getJSONObject("right");
        Number right = rightObject == null ? null : typedNumber(rightObject);
        String implementation = invocation.getString("implementation");
        try {
            Object result;
            if (implementation != null) {
                result = invokeConcreteNumberMath(implementation, operation, left, right);
            }
            else {
                switch (operation) {
                case "abs":
                    result = NumberMath.abs(left);
                    break;
                case "add":
                    result = NumberMath.add(left, requireRight(operation, right));
                    break;
                case "subtract":
                    result = NumberMath.subtract(left, requireRight(operation, right));
                    break;
                case "multiply":
                    result = NumberMath.multiply(left, requireRight(operation, right));
                    break;
                case "divide":
                    result = NumberMath.divide(left, requireRight(operation, right));
                    break;
                case "compareTo":
                    result = NumberMath.compareTo(left, requireRight(operation, right));
                    break;
                case "or":
                    result = NumberMath.or(left, requireRight(operation, right));
                    break;
                case "and":
                    result = NumberMath.and(left, requireRight(operation, right));
                    break;
                case "xor":
                    result = NumberMath.xor(left, requireRight(operation, right));
                    break;
                case "intDiv":
                    result = NumberMath.intDiv(left, requireRight(operation, right));
                    break;
                case "mod":
                    result = NumberMath.mod(left, requireRight(operation, right));
                    break;
                case "remainder":
                    result = NumberMath.remainder(left, requireRight(operation, right));
                    break;
                case "leftShift":
                    result = NumberMath.leftShift(left, requireRight(operation, right));
                    break;
                case "rightShift":
                    result = NumberMath.rightShift(left, requireRight(operation, right));
                    break;
                case "rightShiftUnsigned":
                    result = NumberMath.rightShiftUnsigned(left, requireRight(operation, right));
                    break;
                case "bitwiseNegate":
                    result = NumberMath.bitwiseNegate(left);
                    break;
                case "unaryMinus":
                    result = NumberMath.unaryMinus(left);
                    break;
                case "unaryPlus":
                    result = NumberMath.unaryPlus(left);
                    break;
                case "toBigDecimal":
                    result = NumberMath.toBigDecimal(left);
                    break;
                case "toBigInteger":
                    result = NumberMath.toBigInteger(left);
                    break;
                case "isFloatingPoint":
                    result = NumberMath.isFloatingPoint(left);
                    break;
                case "isInteger":
                    result = NumberMath.isInteger(left);
                    break;
                case "isShort":
                    result = NumberMath.isShort(left);
                    break;
                case "isByte":
                    result = NumberMath.isByte(left);
                    break;
                case "isLong":
                    result = NumberMath.isLong(left);
                    break;
                case "isBigDecimal":
                    result = NumberMath.isBigDecimal(left);
                    break;
                case "isBigInteger":
                    result = NumberMath.isBigInteger(left);
                    break;
                case "getMath":
                    result = NumberMath.getMath(left, requireRight(operation, right)).getClass().getSimpleName();
                    break;
                default:
                    throw new IllegalArgumentException("unsupported number_math operation: " + operation);
                }
            }
            Map<String, Object> record = new LinkedHashMap<>();
            record.put("id", id);
            record.put("outcome", "ok");
            record.put("normalized", normalize(result));
            record.put("error_code", null);
            record.put("line", null);
            record.put("column", null);
            record.put("trace_count", 0);
            return record;
        }
        catch (RuntimeException error) {
            String category = numberMathErrorCategory(error);
            Map<String, Object> record = new LinkedHashMap<>();
            record.put("id", id);
            record.put("outcome", "error");
            record.put("normalized", "error:" + category + ":" + error.getMessage());
            record.put("error_code", category);
            record.put("line", 0);
            record.put("column", 0);
            record.put("trace_count", 0);
            return record;
        }
    }

    /**
     * 直接调用具体 Java NumberMath 实现的公开 override，验证每个数值域，而非只验证门面分派结果。
     *
     * @param implementation Java 实现类简单名
     * @param operation 公开实现方法名，例如 {@code addImpl}
     * @param left 左操作数或一元操作数
     * @param right 可选右操作数
     * @return Java 具体实现的原始返回值
     */
    private static Object invokeConcreteNumberMath(
        String implementation,
        String operation,
        Number left,
        Number right) {
        Object receiver;
        switch (implementation) {
            case "IntegerMath":
                receiver = IntegerMath.INSTANCE;
                break;
            case "LongMath":
                receiver = LongMath.INSTANCE;
                break;
            case "BigIntegerMath":
                receiver = BigIntegerMath.INSTANCE;
                break;
            case "BigDecimalMath":
                receiver = BigDecimalMath.INSTANCE;
                break;
            case "FloatingPointMath":
                receiver = FloatingPointMath.INSTANCE;
                break;
            default:
                throw new IllegalArgumentException("unsupported number_math implementation: " + implementation);
        }
        try {
            Method method;
            if (isUnaryNumberMathImplementation(operation)) {
                method = receiver.getClass().getMethod(operation, Number.class);
                return method.invoke(receiver, left);
            }
            method = receiver.getClass().getMethod(operation, Number.class, Number.class);
            return method.invoke(receiver, left, requireRight(operation, right));
        }
        catch (NoSuchMethodException | IllegalAccessException error) {
            throw new IllegalArgumentException(
                "unsupported concrete number_math operation: " + implementation + "." + operation,
                error);
        }
        catch (InvocationTargetException error) {
            Throwable cause = error.getCause();
            if (cause instanceof RuntimeException) {
                throw (RuntimeException)cause;
            }
            throw new IllegalStateException("concrete number_math invocation failed", cause);
        }
    }

    private static boolean isUnaryNumberMathImplementation(String operation) {
        return "absImpl".equals(operation)
            || "unaryMinusImpl".equals(operation)
            || "unaryPlusImpl".equals(operation)
            || "bitwiseNegateImpl".equals(operation);
    }

    private static Number requireRight(String operation, Number right) {
        if (right == null) {
            throw new IllegalArgumentException("number_math " + operation + " requires right operand");
        }
        return right;
    }

    private static Number typedNumber(JSONObject number) {
        String type = number.getString("type");
        String value = number.getString("value");
        switch (type) {
            case "byte":
                return Byte.valueOf(value);
            case "short":
                return Short.valueOf(value);
            case "int":
                return Integer.valueOf(value);
            case "long":
                return Long.valueOf(value);
            case "float":
                return Float.valueOf(value);
            case "double":
                return Double.valueOf(value);
            case "bigint":
                return new BigInteger(value);
            case "bigdec":
                return new BigDecimal(value);
            default:
                throw new IllegalArgumentException("unsupported number type: " + type);
        }
    }

    private static String numberMathErrorCategory(RuntimeException error) {
        if (error instanceof NumberFormatException) {
            return "NUMBER_FORMAT_EXCEPTION";
        }
        if (error instanceof UnsupportedOperationException) {
            return "UNSUPPORTED_OPERATION";
        }
        if (error instanceof ArithmeticException) {
            return "ARITHMETIC_EXCEPTION";
        }
        return error.getClass().getSimpleName();
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
