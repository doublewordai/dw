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
        """Override wheel tags to produce py3-none-<platform>."""
        def get_tag(self):
            _python, _abi, plat = super().get_tag()
            return "py3", "none", plat

    cmdclass["bdist_wheel"] = BinaryWheel
except ImportError:
    pass


setup(distclass=BinaryDistribution, cmdclass=cmdclass)
