from .griphos import *   # noqa: F403
from .utils import *  # noqa: F403
from enum import IntFlag
from .griphos import SUCCESS, FAILURE, LINEAR_SOLVER_FAILURE, TIMEOUT, MAX_ITER_EXCEEDED

class SolverStatusFlags(IntFlag):
    SUCCESS = SUCCESS
    FAILURE = FAILURE
    LINEAR_SOLVER_FAILURE = LINEAR_SOLVER_FAILURE
    TIMEOUT = TIMEOUT
    MAX_ITER_EXCEEDED = MAX_ITER_EXCEEDED
