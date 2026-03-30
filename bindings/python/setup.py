from setuptools import setup
from setuptools.dist import Distribution


class BinaryDistribution(Distribution):
    """Force platform-specific wheel (not pure Python)."""
    def has_ext_modules(self):
        return True


cmdclass = {}
try:
    from wheel.bdist_wheel import bdist_wheel

    class BinaryWheel(bdist_wheel):
        """Force wheel tags to py3-none-<platform>.

        We bundle a native binary (not a CPython extension), so the wheel
        is compatible with any Python 3 interpreter and has no ABI dependency.
        """
        def finalize_options(self):
            super().finalize_options()
            self.root_is_pure = False

        def get_tag(self):
            _python, _abi, plat = super().get_tag()
            return "py3", "none", plat

    cmdclass["bdist_wheel"] = BinaryWheel
except ImportError:
    pass


setup(distclass=BinaryDistribution, cmdclass=cmdclass)
