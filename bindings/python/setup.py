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
        """Override wheel tags: preserve python tag, force ABI to none."""
        def finalize_options(self):
            super().finalize_options()
            self.root_is_pure = False

        def get_tag(self):
            python, _abi, plat = super().get_tag()
            return python, "none", plat

    cmdclass["bdist_wheel"] = BinaryWheel
except ImportError:
    pass


setup(distclass=BinaryDistribution, cmdclass=cmdclass)
