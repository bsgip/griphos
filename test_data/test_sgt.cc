#include <sgt-e-json/EJson.h>
#include <SgtCore.h>

using namespace Sgt;
using namespace std;

int main(int argc, char** argv)
{
    // -------------------------------------------------------------------------
    // Get options.
    cxxopts::Options options("test_sgt", "Test SGT.");
    options.add_options()("i,input", "e-JSON input filename", cxxopts::value<string>());
    options.add_options()("o,output", "e-JSON output filename", cxxopts::value<string>());
    options.add_options()("s,solver", "Solver, nr_rect | opf_s_pol", cxxopts::value<string>());
    options.add_options()("t,solver_opts", "Solver options", cxxopts::value<string>());
    options.add_options()("w,warm_start", "Warm start");
    options.add_options()("d,debug", "Debug mode", cxxopts::value<string>());
    options.parse_positional({"input"});
    auto args = options.parse(argc, argv);

    const string inputName = args["input"].as<string>();
    std::string outputName = args["output"].as<string>();

    string solver = "nr_rect";
    if (args.count("solver") > 0)
    {
        solver = args["solver"].as<string>();
    }

    json solverOptions = json::object();
    if (args.count("solver_opts") > 0)
    {
        solverOptions = json::parse(args["solver_opts"].as<string>());
    }

    bool warmStart = args.count("warm_start") > 0;

    bool debug = args.count("debug") > 0;

    if (debug)
    {
        const auto& debugArgs = args["debug"].as<string>();
        auto comma = debugArgs.find(",");
        const auto& levelStr = debugArgs.substr(0, comma);
        if (levelStr == "none")
        {
            debugLogLevel() = LogLevel::NONE;
        }
        else if (levelStr == "normal")
        {
            debugLogLevel() = LogLevel::NORMAL;
        }
        else if (levelStr == "verbose")
        {
            debugLogLevel() = LogLevel::VERBOSE;
        }

        if (comma != string::npos)
        {
            debugLogFilter() = debugArgs.substr(comma+1);
        }
    }

    Network netw(1.0, Sgt::VAWOhmUnits);
    parseEJsonFile(netw, inputName);

    if (solver == "nr_rect")
    {
        sgtLogMessage() << "Using nr_rect solver." << endl;
        netw.setSolver(std::make_unique<PowerFlowNrRectSolver>());
    }
    else if (solver == "opf_s_pol")
    {
#ifdef ENABLE_OPF
        sgtLogMessage() << "Using opf_s_pol solver." << endl;
        netw.setSolver(std::make_unique<OpfSPolPfSolver<>>());
#else // ENABLE_OPF
        sgtError("OPF solver is not available, since SmartGridToolbox was not compiled with --enable-madopt.");
#endif // ENABLE_OPF
    }
    else
    {
        sgtError("Solver " << solver << " was not found.");
    }

    // cout << netw << endl;

    netw.solver().setOptions(solverOptions);

    netw.solvePowerFlow(!warmStart);

    auto outfile = ofstream(outputName);
    for (const auto& b : netw.buses())
    {
        for (unsigned i = 0; i < b->phases().size(); ++i)
        {
            outfile << b->id() << ":" << b->phases()[i] << ", " << abs(b->V()[i])/b->VBase() << std::endl;
        };
    }
}
