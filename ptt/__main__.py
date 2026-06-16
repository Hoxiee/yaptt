"""Entry point for ptt daemon."""

from ptt.daemon import PTT


def main():
    PTT().run()


if __name__ == "__main__":
    main()
