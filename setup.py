from setuptools import setup
from setuptools.dist import Distribution
from wheel.bdist_wheel import bdist_wheel as _bdist_wheel


class bdist_wheel(_bdist_wheel):
    def finalize_options(self):
        super().finalize_options()
        self.root_is_pure = False


class BinaryDistribution(Distribution):
    def has_ext_modules(self):
        # This package ships platform-specific native executables.
        return True


setup(cmdclass={"bdist_wheel": bdist_wheel}, distclass=BinaryDistribution)
