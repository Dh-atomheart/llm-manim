import ast
import json
import os
import re
import sys


MAX_CODE_LENGTH = 20000
SCENE_BASES = {"Scene", "MovingCameraScene", "ThreeDScene"}
DEFAULT_ALLOWED_MANIM_NAMES = {
    "Scene",
    "MovingCameraScene",
    "ThreeDScene",
    "Text",
    "MathTex",
    "VGroup",
    "Dot",
    "Write",
    "RIGHT",
}
UNSUPPORTED_MANIM_NAMES = {
    "Color",
    "ParametricSurface",
    "CYAN",
    "MAGENTA",
    "Sequence",
    "OpenGLMobject",
    "OpenGLVMobject",
    "OpenGLGroup",
    "OpenGLVGroup",
    "VoiceoverScene",
}
ANIMATION_SEQUENCE_HELPERS = {"AnimationGroup", "LaggedStart", "Succession"}
TIP_LENGTH_COMPATIBLE_CALLS = {
    "Arrow",
    "DoubleArrow",
    "CurvedArrow",
    "CurvedDoubleArrow",
    "LabeledArrow",
    "Line",
    "DashedLine",
    "Vector",
    "add_tip",
}
BACKGROUND_LINE_STYLE_COMPATIBLE_CALLS = {
    "NumberPlane",
    "ComplexPlane",
}
DENIED_IMPORT_ROOTS = {
    "manimlib",
    "subprocess",
    "shutil",
    "signal",
    "socket",
    "requests",
    "urllib",
    "httpx",
    "pathlib",
    "glob",
    "tempfile",
    "importlib",
    "os",
}
DENIED_CALLS = {"open", "eval", "exec", "compile", "__import__", "input"}
DENIED_ATTRIBUTE_CALLS = {
    ("os", "system"),
    ("os", "popen"),
    ("os", "remove"),
    ("os", "unlink"),
    ("os", "rmdir"),
    ("os", "walk"),
    ("config", "media_dir"),
    ("config", "output_file"),
    ("config", "video_dir"),
    ("config", "tex_dir"),
    ("config", "assets_dir"),
}
DENIED_TEXT_FRAGMENTS = (
    "from manimlib import",
    "import manimlib",
    "interactivescene",
    "manimgl",
    "subprocess",
    "socket",
    "requests",
    "urllib",
    "httpx",
    "open(",
    "eval(",
    "exec(",
    "compile(",
    "__import__",
    "input(",
    "os.system",
    "os.popen",
    "os.remove",
    "os.unlink",
    "os.rmdir",
    "os.walk",
    "pip install",
    "uv run",
)
TEX_MATH_MODE_TOKENS = (
    "\\alpha",
    "\\beta",
    "\\gamma",
    "\\delta",
    "\\epsilon",
    "\\theta",
    "\\lambda",
    "\\mu",
    "\\pi",
    "\\sigma",
    "\\omega",
    "\\Gamma",
    "\\Delta",
    "\\Theta",
    "\\Lambda",
    "\\Pi",
    "\\Sigma",
    "\\Omega",
    "\\frac",
    "\\sqrt",
    "\\sum",
    "\\int",
    "\\partial",
    "\\nabla",
    "\\pm",
    "\\cdot",
    "\\times",
    "\\leq",
    "\\geq",
    "\\neq",
    "\\approx",
    "^",
    "_",
)
PYTHON_BUILTINS = {
    "abs",
    "all",
    "any",
    "bool",
    "dict",
    "enumerate",
    "float",
    "int",
    "len",
    "list",
    "max",
    "min",
    "print",
    "range",
    "round",
    "set",
    "sorted",
    "str",
    "sum",
    "tuple",
    "zip",
}
WINDOWS_PATH_RE = re.compile(r"(?i)^[a-z]:[\\/]")
SHELL_COMMAND_LINE_RE = re.compile(r"(?mi)^\s*manim\s+")


def fail(reason, error_code="E_STATIC_CHECK_FAILED"):
    print(
        json.dumps(
            {"ok": False, "error_code": error_code, "reason": reason},
            ensure_ascii=False,
        )
    )
    return 0


def success(scene_name, normalized_code):
    print(
        json.dumps(
            {
                "ok": True,
                "scene_name": scene_name,
                "normalized_code": normalized_code,
            },
            ensure_ascii=False,
        )
    )
    return 0


def strict_api_name_validation_enabled():
    return os.environ.get("MANIM4LEARN_STRICT_API_NAMES") == "1"


def load_json_file(path):
    with open(path, "r", encoding="utf-8") as file:
        return json.load(file)


def load_compatibility_data(strict_api_names):
    if not strict_api_names:
        return {
            "allowed_names": set(DEFAULT_ALLOWED_MANIM_NAMES),
            "denied_names": set(),
            "denied_import_roots": set(),
            "denied_text_fragments": (),
            "denied_attribute_calls": set(),
            "version": "disabled",
        }

    base_dir = os.environ.get("MANIM4LEARN_MANIMCE_COMPAT_DIR")
    if not base_dir:
        base_dir = os.path.join(os.path.dirname(__file__), "manimce", "0.20.1")

    manifest_path = os.path.join(base_dir, "api_manifest.json")
    denylist_path = os.path.join(base_dir, "denylist.json")
    try:
        manifest = load_json_file(manifest_path)
        denylist = load_json_file(denylist_path)
    except Exception as error:
        raise RuntimeError(str(error)) from error

    allowed_names = set(manifest.get("allowedNames") or [])
    denied_names = set(denylist.get("deniedNames") or [])
    denied_import_roots = set(denylist.get("deniedImportRoots") or [])
    denied_text_fragments = tuple(
        str(fragment).lower() for fragment in (denylist.get("deniedTextFragments") or [])
    )
    denied_attribute_calls = set()
    for value in denylist.get("deniedAttributeCalls") or []:
        if isinstance(value, str) and "." in value:
            owner, name = value.split(".", 1)
            denied_attribute_calls.add((owner, name))

    if not allowed_names:
        raise RuntimeError("api_manifest.json has no allowedNames")

    return {
        "allowed_names": allowed_names,
        "denied_names": denied_names,
        "denied_import_roots": denied_import_roots,
        "denied_text_fragments": denied_text_fragments,
        "denied_attribute_calls": denied_attribute_calls,
        "version": str(manifest.get("version") or "unknown"),
    }


def absolute_path_string(value):
    stripped = value.strip()
    return bool(stripped.startswith("/") or WINDOWS_PATH_RE.match(stripped))


def find_denied_text_fragment(value):
    return find_denied_text_fragment_from(value, DENIED_TEXT_FRAGMENTS)


def find_denied_text_fragment_from(value, fragments):
    lowered = value.lower()
    for fragment in fragments:
        if fragment in lowered:
            return fragment
    return None


def find_disallowed_shell_command_line(value):
    match = SHELL_COMMAND_LINE_RE.search(value)
    if match:
        return match.group(0).strip()
    return None


def extract_base_name(node):
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        return node.attr
    return None


def extract_call_name(node):
    if isinstance(node, ast.Name):
        return (None, node.id)
    if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name):
        return (node.value.id, node.attr)
    return (None, None)


def contains_animation_builder(node):
    if isinstance(node, ast.Attribute) and node.attr == "animate":
        return True
    return any(contains_animation_builder(child) for child in ast.iter_child_nodes(node))


def is_list_with_animation_builder(node):
    return isinstance(node, (ast.List, ast.Tuple)) and any(
        contains_animation_builder(element) for element in node.elts
    )


def has_non_ascii_text_literal(node):
    return isinstance(node, ast.Constant) and isinstance(node.value, str) and any(
        ord(char) > 127 for char in node.value
    )


def has_tex_math_delimiter(value):
    return (
        "$" in value
        or "\\(" in value
        or "\\)" in value
        or "\\[" in value
        or "\\]" in value
        or "\\begin{math}" in value
        or "\\begin{displaymath}" in value
        or "\\begin{equation}" in value
        or "\\begin{align}" in value
    )


def looks_like_undelimited_tex_math(value):
    return (
        any(token in value for token in TEX_MATH_MODE_TOKENS)
        and not has_tex_math_delimiter(value)
    )


class LocalNameCollector(ast.NodeVisitor):
    def __init__(self):
        self.names = set()

    def add_target(self, target):
        if isinstance(target, ast.Name):
            self.names.add(target.id)
        elif isinstance(target, (ast.Tuple, ast.List)):
            for element in target.elts:
                self.add_target(element)

    def visit_FunctionDef(self, node):
        self.names.add(node.name)
        for arg in node.args.posonlyargs + node.args.args + node.args.kwonlyargs:
            self.names.add(arg.arg)
        if node.args.vararg:
            self.names.add(node.args.vararg.arg)
        if node.args.kwarg:
            self.names.add(node.args.kwarg.arg)
        self.generic_visit(node)

    def visit_Lambda(self, node):
        for arg in node.args.posonlyargs + node.args.args + node.args.kwonlyargs:
            self.names.add(arg.arg)
        if node.args.vararg:
            self.names.add(node.args.vararg.arg)
        if node.args.kwarg:
            self.names.add(node.args.kwarg.arg)
        self.generic_visit(node)

    def visit_comprehension(self, node):
        self.add_target(node.target)
        self.generic_visit(node)

    def visit_ClassDef(self, node):
        self.names.add(node.name)
        self.generic_visit(node)

    def visit_Assign(self, node):
        for target in node.targets:
            self.add_target(target)
        self.generic_visit(node)

    def visit_AnnAssign(self, node):
        self.add_target(node.target)
        self.generic_visit(node)

    def visit_AugAssign(self, node):
        self.add_target(node.target)
        self.generic_visit(node)

    def visit_For(self, node):
        self.add_target(node.target)
        self.generic_visit(node)

    def visit_With(self, node):
        for item in node.items:
            if item.optional_vars:
                self.add_target(item.optional_vars)
        self.generic_visit(node)

    def visit_Import(self, node):
        for alias in node.names:
            self.names.add(alias.asname or alias.name.split(".")[0])

    def visit_ImportFrom(self, node):
        for alias in node.names:
            if alias.name != "*":
                self.names.add(alias.asname or alias.name)


class StaticChecker(ast.NodeVisitor):
    def __init__(self, compatibility, local_names, strict_api_names):
        self.errors = []
        self.has_manim_import = False
        self.scene_classes = []
        self.strict_api_names = strict_api_names
        self.allowed_names = compatibility["allowed_names"]
        self.denied_names = compatibility["denied_names"] | UNSUPPORTED_MANIM_NAMES
        self.denied_import_roots = compatibility["denied_import_roots"] | DENIED_IMPORT_ROOTS
        self.denied_attribute_calls = compatibility["denied_attribute_calls"] | DENIED_ATTRIBUTE_CALLS
        self.denied_text_fragments = compatibility["denied_text_fragments"] + DENIED_TEXT_FRAGMENTS
        self.local_names = set(local_names) | {"self", "cls", "np", "math"}

    def error(self, reason):
        self.errors.append(reason)

    def visit_ImportFrom(self, node):
        module = node.module or ""
        root = module.split(".")[0]
        if module == "manim" and any(alias.name == "*" for alias in node.names):
            self.has_manim_import = True
        if module == "manim" and not any(alias.name == "*" for alias in node.names):
            self.error("MANIM_IMPORT_UNSUPPORTED: generated code must use from manim import *")
        if root in self.denied_import_roots:
            self.error(f"MANIMGL_API_DETECTED: generated code contains denied import `{root}`")
        elif root not in {"", "manim", "math", "numpy"}:
            self.error(f"MANIM_IMPORT_UNSUPPORTED: generated code imports unsupported module `{root}`")
        self.generic_visit(node)

    def visit_Import(self, node):
        for alias in node.names:
            root = alias.name.split(".")[0]
            if root in self.denied_import_roots:
                self.error(f"MANIMGL_API_DETECTED: generated code contains denied import `{root}`")
            elif root not in {"math", "numpy"}:
                self.error(f"MANIM_IMPORT_UNSUPPORTED: generated code imports unsupported module `{root}`")
        self.generic_visit(node)

    def visit_ClassDef(self, node):
        base_names = [extract_base_name(base) for base in node.bases]
        if "InteractiveScene" in base_names:
            self.error("生成代码使用了不支持的 InteractiveScene")
        if any(base in SCENE_BASES for base in base_names):
            self.scene_classes.append(node.name)
        self.generic_visit(node)

    def visit_Call(self, node):
        owner, name = extract_call_name(node.func)
        if name in DENIED_CALLS:
            self.error("SECURITY_API_DENIED: generated code calls a denied Python capability")
        if (owner, name) in self.denied_attribute_calls:
            self.error(f"CONFIG_OUTPUT_DENIED: generated code calls denied attribute `{owner}.{name}`")
        if any(keyword.arg == "tip_length" for keyword in node.keywords) and name not in TIP_LENGTH_COMPATIBLE_CALLS:
            self.error(f"MANIM_API_UNSUPPORTED: generated code passes tip_length to `{name}`, which is not a tip-capable ManimCE call")
        if any(keyword.arg == "background_line_style" for keyword in node.keywords) and name not in BACKGROUND_LINE_STYLE_COMPATIBLE_CALLS:
            self.error(f"MANIM_API_UNSUPPORTED: generated code passes background_line_style to `{name}`, but ManimCE 0.20.1 only supports it on NumberPlane or ComplexPlane")
        if name == "len" and node.args and contains_animation_builder(node.args[0]):
            self.error("ANIMATION_BUILDER_MISUSE: generated code calls len(...) on a .animate builder")
        if owner == "self" and name == "play":
            for arg in node.args:
                if is_list_with_animation_builder(arg):
                    self.error("ANIMATION_BUILDER_MISUSE: generated code passes a list of .animate builders to self.play; expand it with *")
        if name in ANIMATION_SEQUENCE_HELPERS:
            for arg in node.args:
                if is_list_with_animation_builder(arg):
                    self.error(f"ANIMATION_BUILDER_MISUSE: generated code passes a list of .animate builders to {name}; expand it with *")
        if name in {"MathTex", "Tex"}:
            for arg in node.args:
                if has_non_ascii_text_literal(arg):
                    self.error("LATEX_RISKY_TEXT: use Text for Chinese or natural-language prose; reserve MathTex/Tex for raw LaTeX formulas")
        if name == "Tex":
            for arg in node.args:
                if isinstance(arg, ast.Constant) and isinstance(arg.value, str) and looks_like_undelimited_tex_math(arg.value):
                    self.error("LATEX_RISKY_TEXT: Tex math content must use explicit math delimiters like $...$; prefer MathTex for pure formulas")
        self.generic_visit(node)

    def visit_Name(self, node):
        if node.id in {"InteractiveScene", "manimgl", "manimlib", "Path"}:
            self.error(f"MANIMGL_API_DETECTED: generated code contains denied identifier `{node.id}`")
        if node.id in self.denied_names:
            self.error(f"MANIM_API_UNSUPPORTED: generated code uses unsupported ManimCE name `{node.id}`")
        elif (
            self.strict_api_names
            and
            isinstance(node.ctx, ast.Load)
            and node.id not in self.local_names
            and node.id not in self.allowed_names
            and node.id not in PYTHON_BUILTINS
        ):
            self.error(f"MANIM_API_UNSUPPORTED: generated code uses name outside the official compatibility manifest `{node.id}`")
        self.generic_visit(node)

    def visit_Attribute(self, node):
        if isinstance(node.ctx, ast.Store) and isinstance(node.value, ast.Name):
            key = (node.value.id, node.attr)
            if key in self.denied_attribute_calls:
                self.error(f"CONFIG_OUTPUT_DENIED: generated code assigns denied attribute `{node.value.id}.{node.attr}`")
        self.generic_visit(node)

    def visit_Constant(self, node):
        if isinstance(node.value, str):
            value = node.value
            if absolute_path_string(value):
                self.error("生成代码包含绝对路径字符串")
            fragment = find_denied_text_fragment_from(value, self.denied_text_fragments)
            if fragment:
                self.error(f"SECURITY_TEXT_DENIED: generated code contains denied text fragment `{fragment}`")
        self.generic_visit(node)


def main():
    try:
        source = sys.stdin.buffer.read().decode("utf-8")
    except UnicodeDecodeError as error:
        return fail("生成代码不是有效 UTF-8 文本: {}".format(error))

    strict_api_names = strict_api_name_validation_enabled()
    try:
        compatibility = load_compatibility_data(strict_api_names)
    except RuntimeError as error:
        return fail(f"MANIM_COMPAT_MANIFEST_UNAVAILABLE: {error}")

    if not source.strip():
        return fail("静态校验输入为空", "E_LLM_OUTPUT_INVALID")

    if len(source) > MAX_CODE_LENGTH:
        return fail("生成代码过长，超过静态校验阈值")

    fragment = find_denied_text_fragment_from(
        source, compatibility["denied_text_fragments"] + DENIED_TEXT_FRAGMENTS
    )
    if fragment:
        return fail(f"SECURITY_TEXT_DENIED: generated code contains denied text fragment `{fragment}`")
    shell_command = find_disallowed_shell_command_line(source)
    if shell_command:
        return fail(f"生成代码包含受限能力或命令片段: denied shell command line `{shell_command}`")

    if any(0xD800 <= ord(char) <= 0xDFFF for char in source):
        return fail("生成代码包含无效 Unicode surrogate 字符")

    try:
        tree = ast.parse(source)
    except SyntaxError as error:
        return fail("生成代码不是有效的 Python 脚本: {}".format(error.msg))

    except UnicodeEncodeError as error:
        return fail("生成代码包含无效 Unicode 文本: {}".format(error))

    collector = LocalNameCollector()
    collector.visit(tree)
    checker = StaticChecker(compatibility, collector.names, strict_api_names)
    checker.visit(tree)

    if not checker.has_manim_import:
        checker.error("生成代码必须包含 from manim import *")

    if len(checker.scene_classes) == 0:
        checker.error("生成代码缺少唯一可渲染 Scene 类")
    elif len(checker.scene_classes) > 1:
        checker.error("生成代码定义了多个 Scene 类")

    if checker.errors:
        unique_errors = []
        for reason in checker.errors:
            if reason not in unique_errors:
                unique_errors.append(reason)
        return fail("；".join(unique_errors))

    normalized_code = source.rstrip() + "\n"
    return success(checker.scene_classes[0], normalized_code)


if __name__ == "__main__":
    sys.exit(main())
