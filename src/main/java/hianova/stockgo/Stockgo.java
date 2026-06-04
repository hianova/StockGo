package hianova.stockgo;

import static java.lang.System.out;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Scanner;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

public class Stockgo {

  private static Manager man;
  private static Selecter sel;
  private static IPFSLayer ipfs;
  private static ArrayList<String>[] data;
  private static Pattern cmdRgx;

  static {
    try {
      man = new Manager();
    } catch (Exception e) {
      out.println("manager not available");
    }
    try {
      ipfs = new IPFSLayer();
    } catch (Exception e) {
      out.println("ipfs not available");
    }
    cmdRgx = Pattern.compile("-\\w+( )*");
  }

  public static void main(String[] args) throws Exception {
    String cmd;
    var exitRgx = Pattern.compile("exit");

    manager("U");
    out.println("Select function:");
    out.println("                -M(manage config) -S(select data) -IPFS(IPFS dedicate)");
    try (var input = new Scanner(System.in)) {
      while (!exitRgx.matcher((cmd = input.nextLine())).find()) {
        var match = cmdRgx.matcher(cmd);
        if (!match.find()) {
          out.println("invalid command");
          continue;
        }
        switch (match.group(0).replaceAll(" -", "")) {
          case "M" -> {
            out.println(man.listConfig());
            out.println("Select \"manage\" function:");
            out.println("                           add(add list) update(update list) delete(del list)\n");
            out.println("                           --help(how to use)");
            while (!exitRgx.matcher((cmd = input.nextLine())).find()) {
              manager(cmd);
            }
          }
          case "S" -> {
            out.println("Select \"select\" function:");
            out.println("                           select(select data) export(export data) test(back test data)");
            out.println("                           view(quick view on data)");
            out.println("                           --help(how to use)");
            while (!exitRgx.matcher((cmd = input.nextLine())).find()) {
              selecter(cmd);
            }
          }
          case "IPFS" -> {
            out.println("Select \"IPFS\" function:");
            out.println("                         import(import list) export(export list)");
            out.println("                         --help(how to use)");
            while (!exitRgx.matcher((cmd = input.nextLine())).find()) {
              IPFSLayer(cmd);
            }
          }
          default -> out.println("command not exist");
        }
        out.println("Select function:");
        out.println("                -M(manage config) -S(select data) -IPFS(IPFS dedicate)");
      }
    }
  }

  public static void manager(String cmdIn) throws Exception {
    var match = cmdRgx.matcher(cmdIn);

    switch (match.find() ? match.group() : "") {
      case "add" -> {
        var map = new HashMap<String, String>();
        var tmp = Arrays.stream(cmdIn.replaceAll("add( )+", "").split(","))
            .map(String::trim).collect(Collectors.toList());
        map.put("title", tmp.get(0));
        map.put("URL", tmp.get(1));
        map.put("label", tmp.get(2));
        map.put("status", tmp.get(3));
        man.addConfig(map);
      }
      case "update" -> man.update();
      case "delete" -> {
        man.deleteConfig(Integer.parseInt(cmdIn.replaceAll("delete( )+", "")));
      }
      case "addRelay" -> {
        var cmd = cmdIn.replaceAll("addRelay( )+", "").split("-");
        var map = new HashMap<String, String>();
        for (var next : cmd) {
          var tmp = next.split("=");
          map.put(tmp[0].trim(), cmd[1].trim());
        }
        man.addRelay(cmd[0].trim(), map);
      }
      case "delRelay" -> {
        var tmp = cmdIn.replaceAll("delRelay( )+", "");
        man.deleteRelay(tmp);
      }
      case "--help" -> helpLayout("manager");
      default -> out.println("command not found");
    }
  }

  public static void IPFSLayer(String cmdIn) throws Exception {
    var match = cmdRgx.matcher(cmdIn);

    switch (match.find() ? match.group(0) : "") {
      case "import" -> {
        ipfs.add(cmdIn.replaceAll("import( )+", ""));
        man.update();
        out.println("file imported");
      }
      case "export" -> out.println(ipfs.share(Integer.parseInt(cmdIn.replaceAll("export( )+", ""))));
      case "--help" -> helpLayout("ipfsLayer");
      default -> out.println("command not found");
    }
  }

  public static void selecter(String cmdIn) throws Exception {
    var match = cmdRgx.matcher(cmdIn);

    switch (match.find() ? match.group(0) : "") {
      case "select" -> {
        var tmp = new ArrayList<>(List.of(cmdIn.replaceAll("select( )+", "").split(",")));
        sel = new Selecter(tmp);
        data = sel.select();
      }
      case "export" -> {
        if (sel == null) {
          out.println("please select data(select) first");
          break;
        }
        sel.export(cmdIn.replaceAll("export( )+", ""), true);
      }
      case "test" -> {
        if (sel == null) {
          out.println("please select data(select) first");
          break;
        }
        out.println(sel.backTest(cmdIn.replaceAll("test( )+", "")));
      }
      case "view" -> {
        if (sel == null) {
          out.println("please select data(select) first");
          break;
        }
        var req = sel.getRequests();
        out.println("\nrequest:");
        IntStream.range(0, req.length).forEach(
            next -> out.println(req[next].subList(0, Math.min(req[next].size(), 10))));
        out.println("\ndata:");
        IntStream.range(0, data.length).forEach(
            next -> out.println(data[next].subList(0, Math.min(data[next].size(), 10))));
      }
      case "--help" -> helpLayout("selecter");
      default -> out.println("command not found");
    }
  }

  public static void helpLayout(String layoutIn) {
    switch (layoutIn) {
      case "manager" -> {
        out.println("\n-M(manage config.txt) page command:");
        out.println("    -A(add list): type in -A [custom title,URL,label,status]");
        out.println("    -U(update list): update to now");
        out.println("    -D(del list): type in -D [number of list]");
      }
      case "selecter" -> {
        out.println("\n-S(select data) page command:");
        out.println(
            "    -D(select data): type in -D [title -request req.req... option:(-date 8digit~8digit)(-numbers num.num...)],[]...");
        out.println("    -E(export data): type in -E [path](default:downloads/exports.csv)");
        out.println("    -T(back test data): type in -T [strategy]");
        out.println("    -detail(quick check on data): preview ten elements per column");
      }
      case "ipfsLayer" -> {
        out.println("\n-IPFS(IPFS dedicate) page command:");
        out.println("    -I(import list): type in -I [CID]");
        out.println("    -O(export list): type in -O [number of list]");
      }
    }
  }
}
